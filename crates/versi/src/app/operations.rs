use std::time::Duration;

use iced::Task;

use crate::message::Message;
use crate::state::{AppState, Operation, OperationRequest, Toast};

use super::Versi;

const INSTALL_TIMEOUT: Duration = Duration::from_secs(600);
const UNINSTALL_TIMEOUT: Duration = Duration::from_secs(60);
const SET_DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

impl Versi {
    pub(super) fn handle_close_modal(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;
        }
    }

    pub(super) fn handle_start_install(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;

            if state.operation_queue.has_active_install(&version)
                || state.operation_queue.has_pending_for_version(&version)
            {
                return Task::none();
            }

            if state.operation_queue.is_busy_for_install() {
                state
                    .operation_queue
                    .enqueue(OperationRequest::Install { version });
                return Task::none();
            }

            return self.start_install_internal(version);
        }
        Task::none()
    }

    pub(super) fn start_install_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.start_install(version.clone());

            let backend = state.backend.clone();

            return Task::perform(
                async move {
                    match tokio::time::timeout(INSTALL_TIMEOUT, backend.install(&version)).await {
                        Ok(Ok(())) => (version, true, None),
                        Ok(Err(e)) => (version, false, Some(e.to_string())),
                        Err(_) => (version, false, Some("Installation timed out".to_string())),
                    }
                },
                |(version, success, error)| Message::InstallComplete {
                    version,
                    success,
                    error,
                },
            );
        }
        Task::none()
    }

    pub(super) fn handle_install_complete(
        &mut self,
        version: String,
        success: bool,
        error: Option<String>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.remove_completed_install(&version);

            if !success {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to install Node {}: {}",
                        version,
                        error.unwrap_or_default()
                    ),
                ));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    pub(super) fn handle_uninstall(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.operation_queue.is_busy_for_exclusive() {
                state
                    .operation_queue
                    .enqueue(OperationRequest::Uninstall { version });
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

            let backend = state.backend.clone();
            let version_clone = version.clone();

            return Task::perform(
                async move {
                    match tokio::time::timeout(UNINSTALL_TIMEOUT, backend.uninstall(&version_clone))
                        .await
                    {
                        Ok(Ok(())) => (version_clone, true, None),
                        Ok(Err(e)) => (version_clone, false, Some(e.to_string())),
                        Err(_) => (
                            version_clone,
                            false,
                            Some("Uninstall timed out".to_string()),
                        ),
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
        version: String,
        success: bool,
        error: Option<String>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.complete_exclusive();

            if !success {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to uninstall Node {}: {}",
                        version,
                        error.unwrap_or_default()
                    ),
                ));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    pub(super) fn handle_set_default(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.operation_queue.is_busy_for_exclusive() {
                state
                    .operation_queue
                    .enqueue(OperationRequest::SetDefault { version });
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

            return Task::perform(
                async move {
                    match tokio::time::timeout(SET_DEFAULT_TIMEOUT, backend.set_default(&version))
                        .await
                    {
                        Ok(Ok(())) => (true, None),
                        Ok(Err(e)) => (false, Some(e.to_string())),
                        Err(_) => (false, Some("Set default timed out".to_string())),
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
        error: Option<String>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.complete_exclusive();

            if !success {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::error(
                    toast_id,
                    format!("Failed to set default: {}", error.unwrap_or_default()),
                ));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    pub(super) fn process_next_operation(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let (install_versions, exclusive_request) = state.operation_queue.drain_next();

            let mut tasks: Vec<Task<Message>> = Vec::new();
            for version in install_versions {
                tasks.push(self.start_install_internal(version));
            }
            if let Some(request) = exclusive_request {
                match request {
                    OperationRequest::Uninstall { version } => {
                        tasks.push(self.start_uninstall_internal(version));
                    }
                    OperationRequest::SetDefault { version } => {
                        tasks.push(self.start_set_default_internal(version));
                    }
                    OperationRequest::Install { .. } => unreachable!(),
                }
            }

            if !tasks.is_empty() {
                return Task::batch(tasks);
            }
        }
        Task::none()
    }
}
