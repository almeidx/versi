//! Application self-update: download, apply, and restart.
//!
//! Handles messages: `StartAppUpdate`, `AppUpdateProgress`, `AppUpdateExtracting`,
//! `AppUpdateApplying`, `AppUpdateComplete`, `RestartApp`

use iced::Task;
use iced::futures::SinkExt;
use log::info;

use versi_core::auto_update::{ApplyResult, UpdateProgress};

use crate::error::AppError;
use crate::message::Message;
use crate::state::{AppState, AppUpdateState};

use super::Versi;

impl Versi {
    pub(super) fn handle_start_app_update(&mut self) -> Task<Message> {
        let AppState::Main(state) = &mut self.state else {
            return Task::none();
        };

        if !matches!(
            state.app_update_state,
            AppUpdateState::Idle | AppUpdateState::Failed(_)
        ) {
            return Task::none();
        }

        let Some(update) = &state.app_update else {
            return Task::none();
        };

        let Some(url) = &update.download_url else {
            return Task::none();
        };

        let url = url.clone();
        let expected_sha256 = update.download_sha256.clone();
        state.app_update_state = AppUpdateState::Downloading {
            downloaded: 0,
            total: update.download_size.unwrap_or(0),
        };

        let client = self.http_client.clone();

        Task::run(
            iced::stream::channel(
                32,
                move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
                    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

                    let download_handle = tokio::spawn(async move {
                        versi_core::auto_update::download_and_apply(
                            &client,
                            &url,
                            expected_sha256.as_deref(),
                            tx,
                        )
                        .await
                    });

                    while let Some(progress) = rx.recv().await {
                        let msg = match progress {
                            UpdateProgress::Downloading { downloaded, total } => {
                                Message::AppUpdateProgress { downloaded, total }
                            }
                            UpdateProgress::Extracting => Message::AppUpdateExtracting,
                            UpdateProgress::Applying => Message::AppUpdateApplying,
                            UpdateProgress::Complete(_) | UpdateProgress::Failed(_) => continue,
                        };
                        let _ = sender.send(msg).await;
                    }

                    let result = match download_handle.await {
                        Ok(result) => {
                            result.map_err(|error| AppError::auto_update_failed("apply", error))
                        }
                        Err(error) => Err(AppError::auto_update_failed(
                            "task join",
                            format!("update task panicked: {error}"),
                        )),
                    };

                    let _ = sender
                        .send(Message::AppUpdateComplete(Box::new(result)))
                        .await;
                },
            ),
            std::convert::identity,
        )
    }

    pub(super) fn handle_app_update_progress(&mut self, downloaded: u64, total: u64) {
        if let AppState::Main(state) = &mut self.state {
            state.app_update_state = AppUpdateState::Downloading { downloaded, total };
        }
    }

    pub(super) fn handle_app_update_extracting(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            state.app_update_state = AppUpdateState::Extracting;
        }
    }

    pub(super) fn handle_app_update_applying(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            state.app_update_state = AppUpdateState::Applying;
        }
    }

    pub(super) fn handle_app_update_complete(
        &mut self,
        result: Result<ApplyResult, AppError>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            match result {
                Ok(ApplyResult::RestartRequired) => {
                    state.app_update_state = AppUpdateState::RestartRequired;
                }
                Ok(ApplyResult::ExitForInstaller) => {
                    return iced::exit();
                }
                Err(e) => {
                    state.app_update_state = AppUpdateState::Failed(e);
                }
            }
        }
        Task::none()
    }

    pub(super) fn handle_restart_app(&mut self) -> Task<Message> {
        info!("Restarting app for update");
        if let Err(error) = versi_core::auto_update::restart_app() {
            if let AppState::Main(state) = &mut self.state {
                state.app_update_state =
                    AppUpdateState::Failed(AppError::auto_update_failed("restart", error));
            }
            return Task::none();
        }
        iced::exit()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_app_with_two_environments;
    use super::*;
    use versi_core::AppUpdate;

    fn sample_update(download_url: Option<&str>, download_size: Option<u64>) -> AppUpdate {
        AppUpdate {
            current_version: "0.9.0".to_string(),
            latest_version: "0.10.0".to_string(),
            release_url: "https://example.com/release".to_string(),
            release_notes: None,
            download_url: download_url.map(ToString::to_string),
            download_size,
            download_sha256: Some(
                "50639d63848d275a7efcd04478de62ca0df8f35dfd75be490e4fcae667ecd436".to_string(),
            ),
        }
    }

    #[test]
    fn start_app_update_sets_downloading_when_update_is_ready() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.app_update = Some(sample_update(
            Some("https://example.com/download.zip"),
            Some(42),
        ));
        state.app_update_state = AppUpdateState::Idle;

        let _ = app.handle_start_app_update();

        let state = app.main_state();
        assert!(matches!(
            state.app_update_state,
            AppUpdateState::Downloading {
                downloaded: 0,
                total: 42
            }
        ));
    }

    #[test]
    fn start_app_update_keeps_state_when_not_ready() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.app_update = Some(sample_update(None, Some(5)));
        state.app_update_state = AppUpdateState::Idle;

        let _ = app.handle_start_app_update();

        let state = app.main_state();
        assert!(matches!(state.app_update_state, AppUpdateState::Idle));

        let state = app.main_state_mut();
        state.app_update = Some(sample_update(
            Some("https://example.com/download.zip"),
            Some(5),
        ));
        state.app_update_state = AppUpdateState::Applying;

        let _ = app.handle_start_app_update();

        let state = app.main_state();
        assert!(matches!(state.app_update_state, AppUpdateState::Applying));
    }

    #[test]
    fn app_update_progress_handlers_update_state_variants() {
        let mut app = test_app_with_two_environments();

        app.handle_app_update_progress(10, 100);
        let state = app.main_state();
        assert!(matches!(
            state.app_update_state,
            AppUpdateState::Downloading {
                downloaded: 10,
                total: 100
            }
        ));

        app.handle_app_update_extracting();
        let state = app.main_state();
        assert!(matches!(state.app_update_state, AppUpdateState::Extracting));

        app.handle_app_update_applying();
        let state = app.main_state();
        assert!(matches!(state.app_update_state, AppUpdateState::Applying));
    }

    #[test]
    fn app_update_complete_sets_restart_required_or_failed() {
        let mut app = test_app_with_two_environments();
        let _ = app.handle_app_update_complete(Ok(ApplyResult::RestartRequired));

        let state = app.main_state();
        assert!(matches!(
            state.app_update_state,
            AppUpdateState::RestartRequired
        ));

        let mut app = test_app_with_two_environments();
        let _ = app
            .handle_app_update_complete(Err(AppError::auto_update_failed("apply", "apply failed")));
        let state = app.main_state();
        assert!(matches!(
            &state.app_update_state,
            AppUpdateState::Failed(AppError::AutoUpdateFailed { phase, details })
                if phase == &"apply"
                    && details == &crate::error::AppErrorDetail::from("apply failed")
        ));
    }

    #[test]
    fn start_app_update_noop_when_no_update_available() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut().app_update = None;
        app.main_state_mut().app_update_state = AppUpdateState::Idle;

        let _ = app.handle_start_app_update();

        assert!(matches!(
            app.main_state().app_update_state,
            AppUpdateState::Idle
        ));
    }

    #[test]
    fn start_app_update_retries_from_failed_state() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.app_update = Some(sample_update(
            Some("https://example.com/download.zip"),
            Some(99),
        ));
        state.app_update_state =
            AppUpdateState::Failed(AppError::auto_update_failed("apply", "disk full"));

        let _ = app.handle_start_app_update();

        assert!(matches!(
            app.main_state().app_update_state,
            AppUpdateState::Downloading {
                downloaded: 0,
                total: 99
            }
        ));
    }

    #[test]
    fn start_app_update_blocked_for_non_retriable_states() {
        for initial_state in [
            AppUpdateState::Downloading {
                downloaded: 0,
                total: 0,
            },
            AppUpdateState::Extracting,
            AppUpdateState::RestartRequired,
        ] {
            let mut app = test_app_with_two_environments();
            let state = app.main_state_mut();
            state.app_update = Some(sample_update(
                Some("https://example.com/download.zip"),
                Some(42),
            ));
            state.app_update_state = initial_state;

            let _ = app.handle_start_app_update();

            let state = app.main_state();
            assert!(
                !matches!(
                    state.app_update_state,
                    AppUpdateState::Downloading {
                        downloaded: 0,
                        total: 42
                    }
                ),
                "should not transition to Downloading from non-retriable state"
            );
        }
    }

    #[test]
    fn start_app_update_uses_zero_when_download_size_is_none() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.app_update = Some(sample_update(
            Some("https://example.com/download.zip"),
            None,
        ));
        state.app_update_state = AppUpdateState::Idle;

        let _ = app.handle_start_app_update();

        assert!(matches!(
            app.main_state().app_update_state,
            AppUpdateState::Downloading {
                downloaded: 0,
                total: 0
            }
        ));
    }

    #[test]
    fn app_update_progress_overwrites_prior_state() {
        let mut app = test_app_with_two_environments();

        app.handle_app_update_progress(10, 100);
        app.handle_app_update_progress(50, 100);

        assert!(matches!(
            app.main_state().app_update_state,
            AppUpdateState::Downloading {
                downloaded: 50,
                total: 100
            }
        ));
    }

    #[test]
    fn app_update_complete_failed_preserves_phase_string() {
        let mut app = test_app_with_two_environments();
        let _ = app.handle_app_update_complete(Err(AppError::auto_update_failed(
            "task join",
            "update task panicked: JoinError",
        )));

        assert!(matches!(
            &app.main_state().app_update_state,
            AppUpdateState::Failed(AppError::AutoUpdateFailed { phase, .. })
                if phase == &"task join"
        ));
    }
}
