//! Install, uninstall, and set-default operations with queuing.
//!
//! Handles messages: `StartInstall`, `InstallComplete`, Uninstall, `UninstallComplete`,
//! `SetDefault`, `DefaultChanged`, `CloseModal`

use std::collections::HashSet;
use std::time::Duration;

use iced::Task;
use iced::futures::SinkExt;
use versi_backend::{InstallProgress, NodeVersion};

use crate::error::AppError;
use crate::message::Message;
use crate::state::{AppState, BulkRunAction, MainState, Modal, Operation, Toast};

use super::Versi;
use super::async_helpers::run_with_timeout;

fn has_duplicate_install_request(state: &MainState, version: &str) -> bool {
    state.operation_queue.has_active_install(version)
        || state.operation_queue.has_pending_for_version(version)
}

fn enqueue_install_if_busy(state: &mut MainState, version: &str) -> bool {
    if state.operation_queue.is_busy_for_install() {
        state.operation_queue.enqueue(Operation::Install {
            version: version.to_string(),
        });
        return true;
    }
    false
}

fn enqueue_exclusive_if_busy(state: &mut MainState, request: Operation) -> bool {
    if state.operation_queue.is_busy_for_exclusive() {
        state.operation_queue.enqueue(request);
        return true;
    }
    false
}

fn should_confirm_default_uninstall(state: &MainState, version: &str) -> bool {
    let Ok(version) = version.parse::<NodeVersion>() else {
        return false;
    };
    state
        .active_environment()
        .default_version
        .as_ref()
        .is_some_and(|dv| dv == &version)
}

fn is_already_default_version(state: &MainState, version: &str) -> bool {
    let Ok(version) = version.parse::<NodeVersion>() else {
        return false;
    };
    state
        .active_environment()
        .default_version
        .as_ref()
        .is_some_and(|dv| dv == &version)
}

fn error_text(error: Option<AppError>) -> String {
    error.map_or_else(|| "unknown error".to_string(), |e| e.to_string())
}

fn install_failure_message(version: &str, error: Option<AppError>) -> String {
    format!("Failed to install Node {version}: {}", error_text(error))
}

fn uninstall_failure_message(version: &str, error: Option<AppError>) -> String {
    format!("Failed to uninstall Node {version}: {}", error_text(error))
}

fn set_default_failure_message(error: Option<AppError>) -> String {
    format!("Failed to set default: {}", error_text(error))
}

fn bulk_operation_error(operation: &'static str, error: Option<AppError>) -> AppError {
    error.unwrap_or_else(|| AppError::operation_failed(operation, "unknown error"))
}

fn add_failure_toast(state: &mut MainState, message: String) {
    let toast_id = state.next_toast_id();
    state.add_toast(Toast::error(toast_id, message));
}

fn mark_bulk_item_running(state: &mut MainState, version: &str, action: BulkRunAction) {
    if let Some(run) = state.bulk_run.as_mut() {
        run.mark_running(version, action);
    }
}

fn mark_bulk_item_finished(
    state: &mut MainState,
    version: &str,
    action: BulkRunAction,
    success: bool,
    error: Option<AppError>,
) {
    if let Some(run) = state.bulk_run.as_mut() {
        let bulk_error = (!success).then(|| {
            let operation = match action {
                BulkRunAction::Install => "Install",
                BulkRunAction::Uninstall => "Uninstall",
            };
            bulk_operation_error(operation, error)
        });
        run.mark_finished(version, action, success, bulk_error);
    }
}

fn clear_inactive_bulk_run(state: &mut MainState) {
    if state.bulk_run.as_ref().is_some_and(|run| !run.is_active()) {
        state.bulk_run = None;
    }
}

const BULK_INSTALL_DISPATCH_LIMIT: usize = 1;

impl Versi {
    pub(super) fn handle_close_modal(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;
        }
    }

    pub(super) fn handle_start_install(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;

            if has_duplicate_install_request(state, &version) {
                return Task::none();
            }

            if enqueue_install_if_busy(state, &version) {
                return Task::none();
            }

            return self.start_install_internal(version);
        }
        Task::none()
    }

    pub(super) fn handle_cancel_bulk_run(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(run) = state.bulk_run.as_mut()
        {
            let canceled = run.cancel_pending();
            if canceled.is_empty() {
                return Task::none();
            }

            let mut install_targets = HashSet::new();
            let mut uninstall_targets = HashSet::new();
            for (version, action) in canceled {
                match action {
                    BulkRunAction::Install => {
                        install_targets.insert(version);
                    }
                    BulkRunAction::Uninstall => {
                        uninstall_targets.insert(version);
                    }
                }
            }

            let _removed = state
                .operation_queue
                .remove_pending_matching(|op| match op {
                    Operation::Install { version } => install_targets.contains(version),
                    Operation::Uninstall { version } => uninstall_targets.contains(version),
                    Operation::SetDefault { .. } => false,
                });

            clear_inactive_bulk_run(state);
        }
        Task::none()
    }

    pub(super) fn start_install_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.start_install(version.clone());
            state.install_progress.remove(&version);
            mark_bulk_item_running(state, &version, BulkRunAction::Install);

            let backend = state.backend.clone();
            let timeout = Duration::from_secs(self.settings.install_timeout_secs);

            return Task::run(
                iced::stream::channel(
                    32,
                    move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
                        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(64);
                        let install_version = version.clone();

                        let install_task = tokio::spawn(async move {
                            run_with_timeout(
                                timeout,
                                "Installation",
                                backend.install_with_progress(&install_version, progress_tx),
                                |error| AppError::operation_failed("Install", error),
                            )
                            .await
                        });

                        while let Some(progress) = progress_rx.recv().await {
                            let _ = sender
                                .send(Message::InstallProgress {
                                    version: version.clone(),
                                    progress,
                                })
                                .await;
                        }

                        let result = match install_task.await {
                            Ok(Ok(())) => (version, true, None),
                            Ok(Err(error)) => (version, false, Some(error)),
                            Err(error) => (
                                version,
                                false,
                                Some(AppError::operation_failed(
                                    "Install",
                                    format!("install task panicked: {error}"),
                                )),
                            ),
                        };

                        let _ = sender
                            .send(Message::InstallComplete {
                                version: result.0,
                                success: result.1,
                                error: result.2,
                            })
                            .await;
                    },
                ),
                std::convert::identity,
            );
        }
        Task::none()
    }

    pub(super) fn handle_install_progress(
        &mut self,
        version: String,
        progress: InstallProgress,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && state.operation_queue.has_active_install(&version)
        {
            state.install_progress.insert(version, progress);
        }
        Task::none()
    }

    pub(super) fn handle_install_complete(
        &mut self,
        version: &str,
        success: bool,
        error: Option<AppError>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.remove_completed_install(version);
            state.install_progress.remove(version);
            mark_bulk_item_finished(
                state,
                version,
                BulkRunAction::Install,
                success,
                error.clone(),
            );

            if !success {
                add_failure_toast(state, install_failure_message(version, error));
            }

            clear_inactive_bulk_run(state);
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    pub(super) fn handle_uninstall(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if should_confirm_default_uninstall(state, &version) {
                state.modal = Some(Modal::ConfirmUninstallDefault {
                    version: version.clone(),
                });
                return Task::none();
            }

            if enqueue_exclusive_if_busy(
                state,
                Operation::Uninstall {
                    version: version.clone(),
                },
            ) {
                return Task::none();
            }

            return self.start_uninstall_internal(version);
        }
        Task::none()
    }

    pub(super) fn handle_confirm_uninstall_default(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;

            if enqueue_exclusive_if_busy(
                state,
                Operation::Uninstall {
                    version: version.clone(),
                },
            ) {
                return Task::none();
            }

            return self.start_uninstall_internal(version);
        }
        Task::none()
    }

    pub(super) fn start_uninstall_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.start_exclusive(Operation::Uninstall {
                version: version.clone(),
            });
            mark_bulk_item_running(state, &version, BulkRunAction::Uninstall);

            let backend = state.backend.clone();
            let timeout = Duration::from_secs(self.settings.uninstall_timeout_secs);

            return Task::perform(
                async move {
                    match run_with_timeout(
                        timeout,
                        "Uninstall",
                        backend.uninstall(&version),
                        |error| AppError::operation_failed("Uninstall", error),
                    )
                    .await
                    {
                        Ok(()) => (version, true, None),
                        Err(error) => (version, false, Some(error)),
                    }
                },
                |(version, success, error)| Message::UninstallComplete {
                    version,
                    success,
                    error,
                },
            );
        }
        Task::none()
    }

    pub(super) fn handle_uninstall_complete(
        &mut self,
        version: &str,
        success: bool,
        error: Option<AppError>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.complete_exclusive();
            mark_bulk_item_finished(
                state,
                version,
                BulkRunAction::Uninstall,
                success,
                error.clone(),
            );

            if !success {
                add_failure_toast(state, uninstall_failure_message(version, error));
            }

            clear_inactive_bulk_run(state);
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    pub(super) fn handle_set_default(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if is_already_default_version(state, &version) {
                return Task::none();
            }

            if enqueue_exclusive_if_busy(
                state,
                Operation::SetDefault {
                    version: version.clone(),
                },
            ) {
                return Task::none();
            }

            return self.start_set_default_internal(version);
        }
        Task::none()
    }

    pub(super) fn start_set_default_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state
                .operation_queue
                .start_exclusive(Operation::SetDefault {
                    version: version.clone(),
                });

            let backend = state.backend.clone();
            let timeout = Duration::from_secs(self.settings.set_default_timeout_secs);

            return Task::perform(
                async move {
                    match run_with_timeout(
                        timeout,
                        "Set default",
                        backend.set_default(&version),
                        |error| AppError::operation_failed("Set default", error),
                    )
                    .await
                    {
                        Ok(()) => (true, None),
                        Err(error) => (false, Some(error)),
                    }
                },
                |(success, error)| Message::DefaultChanged { success, error },
            );
        }
        Task::none()
    }

    pub(super) fn handle_default_changed(
        &mut self,
        success: bool,
        error: Option<AppError>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.complete_exclusive();

            if !success {
                add_failure_toast(state, set_default_failure_message(error));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    pub(super) fn process_next_operation(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let install_limit = state.bulk_run.as_ref().and_then(|run| {
                if run.is_active() {
                    Some(BULK_INSTALL_DISPATCH_LIMIT)
                } else {
                    None
                }
            });
            let (install_versions, exclusive_request) = if let Some(limit) = install_limit {
                state.operation_queue.drain_next_with_limit(Some(limit))
            } else {
                state.operation_queue.drain_next()
            };

            let mut tasks: Vec<Task<Message>> = Vec::new();
            for version in install_versions {
                tasks.push(self.start_install_internal(version));
            }
            if let Some(request) = exclusive_request {
                tasks.push(self.task_for_exclusive_request(request));
            }

            if !tasks.is_empty() {
                return Task::batch(tasks);
            }
        }
        Task::none()
    }

    fn task_for_exclusive_request(&mut self, request: Operation) -> Task<Message> {
        match request {
            Operation::Uninstall { version } => self.start_uninstall_internal(version),
            Operation::SetDefault { version } => self.start_set_default_internal(version),
            Operation::Install { .. } => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use versi_backend::InstallProgress;

    use super::super::test_app_with_two_environments;
    use super::*;

    #[test]
    fn close_modal_clears_existing_modal() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut().modal = Some(Modal::KeyboardShortcuts);

        app.handle_close_modal();

        let state = app.main_state();
        assert!(state.modal.is_none());
    }

    #[test]
    fn start_install_ignores_duplicate_active_version() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut()
            .operation_queue
            .start_install("v20.11.0".to_string());

        let _ = app.handle_start_install("v20.11.0".to_string());

        let state = app.main_state();
        assert_eq!(state.operation_queue.active_installs.len(), 1);
        assert!(state.operation_queue.pending.is_empty());
    }

    #[test]
    fn start_install_queues_when_exclusive_operation_is_active() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut()
            .operation_queue
            .start_exclusive(Operation::SetDefault {
                version: "v20.11.0".to_string(),
            });

        let _ = app.handle_start_install("v22.1.0".to_string());

        let state = app.main_state();
        assert_eq!(state.operation_queue.pending.len(), 1);
        assert!(matches!(
            state.operation_queue.pending.front(),
            Some(Operation::Install { version }) if version == "v22.1.0"
        ));
    }

    #[test]
    fn uninstall_default_opens_confirmation_modal() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut()
            .active_environment_mut()
            .default_version = Some(
            "v20.11.0"
                .parse()
                .expect("test default version should parse"),
        );

        let _ = app.handle_uninstall("v20.11.0".to_string());

        let state = app.main_state();
        assert!(matches!(
            state.modal,
            Some(Modal::ConfirmUninstallDefault { ref version }) if version == "v20.11.0"
        ));
    }

    #[test]
    fn uninstall_queues_when_exclusive_queue_is_busy() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.active_environment_mut().default_version = None;
        state.operation_queue.start_exclusive(Operation::Uninstall {
            version: "v18.0.0".to_string(),
        });

        let _ = app.handle_uninstall("v20.11.0".to_string());

        let state = app.main_state();
        assert!(matches!(
            state.operation_queue.pending.front(),
            Some(Operation::Uninstall { version }) if version == "v20.11.0"
        ));
    }

    #[test]
    fn set_default_queues_when_exclusive_queue_is_busy() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut()
            .operation_queue
            .start_exclusive(Operation::Uninstall {
                version: "v18.0.0".to_string(),
            });

        let _ = app.handle_set_default("v22.0.0".to_string());

        let state = app.main_state();
        assert!(matches!(
            state.operation_queue.pending.front(),
            Some(Operation::SetDefault { version }) if version == "v22.0.0"
        ));
    }

    #[test]
    fn set_default_is_noop_when_version_is_already_default() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.active_environment_mut().default_version = Some(
            "v22.0.0"
                .parse()
                .expect("test default version should parse"),
        );

        let _ = app.handle_set_default("v22.0.0".to_string());

        let state = app.main_state();
        assert!(state.operation_queue.exclusive_op.is_none());
        assert!(state.operation_queue.pending.is_empty());
    }

    #[test]
    fn install_progress_updates_active_install_and_is_cleared_on_completion() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut()
            .operation_queue
            .start_install("v22.1.0".to_string());

        let _ = app.handle_install_progress(
            "v22.1.0".to_string(),
            InstallProgress::Downloading {
                downloaded_bytes: 5,
                total_bytes: 10,
            },
        );

        let state = app.main_state();
        assert!(matches!(
            state.install_progress.get("v22.1.0"),
            Some(InstallProgress::Downloading {
                downloaded_bytes: 5,
                total_bytes: 10
            })
        ));

        let _ = app.handle_install_complete("v22.1.0", true, None);
        let state = app.main_state();
        assert!(!state.install_progress.contains_key("v22.1.0"));
    }

    #[test]
    fn cancel_bulk_run_removes_only_pending_bulk_operations() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.bulk_run = Some(crate::state::BulkRunState::new(
            crate::state::BulkRunKind::UpdateMajors,
            vec![
                crate::state::BulkRunItem {
                    version: "v22.1.0".to_string(),
                    action: crate::state::BulkRunAction::Install,
                    status: crate::state::BulkItemStatus::Pending,
                },
                crate::state::BulkRunItem {
                    version: "v18.19.0".to_string(),
                    action: crate::state::BulkRunAction::Uninstall,
                    status: crate::state::BulkItemStatus::Pending,
                },
                crate::state::BulkRunItem {
                    version: "v20.11.0".to_string(),
                    action: crate::state::BulkRunAction::Install,
                    status: crate::state::BulkItemStatus::Running,
                },
            ],
        ));
        state.operation_queue.enqueue(Operation::Install {
            version: "v22.1.0".to_string(),
        });
        state.operation_queue.enqueue(Operation::Uninstall {
            version: "v18.19.0".to_string(),
        });
        state.operation_queue.enqueue(Operation::Install {
            version: "v16.20.0".to_string(),
        });
        state.operation_queue.enqueue(Operation::SetDefault {
            version: "v16.20.0".to_string(),
        });

        let _ = app.handle_cancel_bulk_run();

        let state = app.main_state();
        assert_eq!(state.operation_queue.pending.len(), 2);
        assert!(matches!(
            state.operation_queue.pending.front(),
            Some(Operation::Install { version }) if version == "v16.20.0"
        ));
        assert!(state.bulk_run.is_some());
        let run = state
            .bulk_run
            .as_ref()
            .expect("bulk run should remain while running");
        assert_eq!(run.pending_count(), 0);
        assert_eq!(run.running_count(), 1);
        assert_eq!(run.canceled_count(), 2);
    }

    #[test]
    fn process_next_operation_limits_bulk_install_dispatch() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.bulk_run = Some(crate::state::BulkRunState::new(
            crate::state::BulkRunKind::UpdateMajors,
            vec![
                crate::state::BulkRunItem {
                    version: "v22.1.0".to_string(),
                    action: crate::state::BulkRunAction::Install,
                    status: crate::state::BulkItemStatus::Pending,
                },
                crate::state::BulkRunItem {
                    version: "v20.11.1".to_string(),
                    action: crate::state::BulkRunAction::Install,
                    status: crate::state::BulkItemStatus::Pending,
                },
                crate::state::BulkRunItem {
                    version: "v18.20.0".to_string(),
                    action: crate::state::BulkRunAction::Install,
                    status: crate::state::BulkItemStatus::Pending,
                },
            ],
        ));
        state.operation_queue.enqueue(Operation::Install {
            version: "v22.1.0".to_string(),
        });
        state.operation_queue.enqueue(Operation::Install {
            version: "v20.11.1".to_string(),
        });
        state.operation_queue.enqueue(Operation::Install {
            version: "v18.20.0".to_string(),
        });

        let _ = app.process_next_operation();

        let state = app.main_state();
        assert_eq!(state.operation_queue.active_installs.len(), 1);
        assert_eq!(state.operation_queue.pending.len(), 2);
    }
}
