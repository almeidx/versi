use iced::Task;
use iced::futures::SinkExt;
use log::info;

use versi_core::auto_update::{ApplyResult, UpdateProgress};

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
                        versi_core::auto_update::download_and_apply(&client, &url, tx).await
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
                        Ok(r) => r,
                        Err(e) => Err(format!("Update task panicked: {e}")),
                    };

                    let _ = sender.send(Message::AppUpdateComplete(result)).await;
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
        result: Result<ApplyResult, String>,
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
        if let Err(e) = versi_core::auto_update::restart_app() {
            if let AppState::Main(state) = &mut self.state {
                state.app_update_state = AppUpdateState::Failed(format!("Restart failed: {e}"));
            }
            return Task::none();
        }
        iced::exit()
    }
}
