use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::time::Duration;

use crate::settings::AppSettings;

const SETTINGS_SAVE_DEBOUNCE: Duration = Duration::from_millis(250);

struct QueuedSettingsSave {
    settings: AppSettings,
    generation: u64,
}

struct SettingsSaveState {
    generation: AtomicU64,
    write_lock: Mutex<()>,
}

struct SettingsSaveCoordinator {
    sender: mpsc::Sender<QueuedSettingsSave>,
    state: Arc<SettingsSaveState>,
}

pub(super) fn enqueue_settings_save(settings: AppSettings) {
    let coordinator = settings_save_coordinator();
    let queued = QueuedSettingsSave {
        settings,
        generation: coordinator.state.generation.load(Ordering::Acquire),
    };
    let _ = coordinator.sender.send(queued);
}

pub(super) fn save_settings_sync(settings: &AppSettings) -> Result<(), std::io::Error> {
    let coordinator = settings_save_coordinator();
    let _guard = coordinator
        .state
        .write_lock
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let result = settings.save();
    if result.is_ok() {
        coordinator.state.generation.fetch_add(1, Ordering::AcqRel);
    }
    result
}

fn settings_save_coordinator() -> &'static SettingsSaveCoordinator {
    static SETTINGS_SAVER: OnceLock<SettingsSaveCoordinator> = OnceLock::new();

    SETTINGS_SAVER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<QueuedSettingsSave>();
        let state = Arc::new(SettingsSaveState {
            generation: AtomicU64::new(0),
            write_lock: Mutex::new(()),
        });
        let worker_state = Arc::clone(&state);
        std::thread::spawn(move || {
            while let Ok(mut latest) = receiver.recv() {
                loop {
                    match receiver.recv_timeout(SETTINGS_SAVE_DEBOUNCE) {
                        Ok(next) => latest = next,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            save_latest_if_current(&worker_state, &latest);
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            save_latest_if_current(&worker_state, &latest);
                            return;
                        }
                    }
                }
            }
        });
        SettingsSaveCoordinator { sender, state }
    })
}

fn save_latest_if_current(state: &SettingsSaveState, latest: &QueuedSettingsSave) {
    let _guard = state
        .write_lock
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if latest.generation != state.generation.load(Ordering::Acquire) {
        return;
    }

    if let Err(error) = latest.settings.save() {
        log::error!("Failed to save settings: {error}");
    }
}
