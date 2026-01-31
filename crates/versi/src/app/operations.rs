use iced::Task;

use crate::message::Message;
use crate::state::{AppState, Modal, Operation, OperationRequest, QueuedOperation, Toast};

use super::Versi;

impl Versi {
    pub(super) fn handle_close_modal(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;
        }
    }

    pub(super) fn handle_start_install(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;

            if state
                .operation_queue
                .active_installs
                .iter()
                .any(|op| matches!(op, Operation::Install { version: v, .. } if v == &version))
                || state.operation_queue.has_pending_for_version(&version)
            {
                return Task::none();
            }

            if state.operation_queue.is_busy_for_install() {
                state.operation_queue.pending.push_back(QueuedOperation {
                    request: OperationRequest::Install {
                        version: version.clone(),
                    },
                });
                return Task::none();
            }

            return self.start_install_internal(version);
        }
        Task::none()
    }

    pub(super) fn start_install_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state
                .operation_queue
                .active_installs
                .push(Operation::Install {
                    version: version.clone(),
                    progress: Default::default(),
                });

            let backend = state.backend.clone();
            let version_clone = version.clone();

            let install_stream = async_stream::stream! {
                match backend.install_with_progress(&version_clone).await {
                    Ok(mut rx) => {
                        let mut final_success = false;
                        let mut last_error: Option<String> = None;
                        while let Some(progress) = rx.recv().await {
                            let is_complete = progress.phase == versi_backend::InstallPhase::Complete;
                            let is_failed = progress.phase == versi_backend::InstallPhase::Failed;

                            if is_failed {
                                last_error = progress.error.clone();
                            }

                            yield Message::InstallProgress {
                                version: version_clone.clone(),
                                progress,
                            };

                            if is_complete {
                                final_success = true;
                                break;
                            }
                            if is_failed {
                                break;
                            }
                        }
                        yield Message::InstallComplete {
                            version: version_clone.clone(),
                            success: final_success,
                            error: if final_success { None } else { last_error.or_else(|| Some("Installation failed".to_string())) },
                        };
                    }
                    Err(e) => {
                        yield Message::InstallComplete {
                            version: version_clone.clone(),
                            success: false,
                            error: Some(e.to_string()),
                        };
                    }
                }
            };
            return Task::run(install_stream, |msg| msg);
        }
        Task::none()
    }

    pub(super) fn handle_install_progress(
        &mut self,
        version: String,
        progress: versi_backend::InstallProgress,
    ) {
        if let AppState::Main(state) = &mut self.state {
            state
                .operation_queue
                .update_install_progress(&version, progress);
        }
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
                state.operation_queue.pending.push_back(QueuedOperation {
                    request: OperationRequest::Uninstall {
                        version: version.clone(),
                    },
                });
                return Task::none();
            }

            return self.start_uninstall_internal(version);
        }
        Task::none()
    }

    pub(super) fn start_uninstall_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.exclusive_op = Some(Operation::Uninstall {
                version: version.clone(),
            });

            let backend = state.backend.clone();
            let version_clone = version.clone();

            return Task::perform(
                async move {
                    match backend.uninstall(&version_clone).await {
                        Ok(()) => (version_clone, true, None),
                        Err(e) => (version_clone, false, Some(e.to_string())),
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
            state.operation_queue.exclusive_op = None;

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
                state.operation_queue.pending.push_back(QueuedOperation {
                    request: OperationRequest::SetDefault {
                        version: version.clone(),
                    },
                });
                return Task::none();
            }

            return self.start_set_default_internal(version);
        }
        Task::none()
    }

    pub(super) fn start_set_default_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.exclusive_op = Some(Operation::SetDefault {
                version: version.clone(),
            });

            let backend = state.backend.clone();

            return Task::perform(
                async move {
                    match backend.set_default(&version).await {
                        Ok(()) => (true, None),
                        Err(e) => (false, Some(e.to_string())),
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
            state.operation_queue.exclusive_op = None;

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

    pub(super) fn handle_request_bulk_update_majors(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment();
            let remote = &state.available_versions.versions;

            let latest_remote_by_major: std::collections::HashMap<u32, versi_backend::NodeVersion> = {
                let mut latest = std::collections::HashMap::new();
                for v in remote {
                    let major = v.version.major;
                    latest
                        .entry(major)
                        .and_modify(|existing: &mut versi_backend::NodeVersion| {
                            if v.version > *existing {
                                *existing = v.version.clone();
                            }
                        })
                        .or_insert_with(|| v.version.clone());
                }
                latest
            };

            let latest_installed_by_major: std::collections::HashMap<
                u32,
                versi_backend::NodeVersion,
            > = {
                let mut latest = std::collections::HashMap::new();
                for v in &env.installed_versions {
                    let major = v.version.major;
                    latest
                        .entry(major)
                        .and_modify(|existing: &mut versi_backend::NodeVersion| {
                            if v.version > *existing {
                                *existing = v.version.clone();
                            }
                        })
                        .or_insert_with(|| v.version.clone());
                }
                latest
            };

            let versions_to_update: Vec<(String, String)> = latest_installed_by_major
                .iter()
                .filter_map(|(major, installed)| {
                    latest_remote_by_major.get(major).and_then(|latest| {
                        if latest > installed {
                            Some((installed.to_string(), latest.to_string()))
                        } else {
                            None
                        }
                    })
                })
                .collect();

            if versions_to_update.is_empty() {
                return Task::none();
            }

            state.modal = Some(Modal::ConfirmBulkUpdateMajors {
                versions: versions_to_update,
            });
        }
        Task::none()
    }

    pub(super) fn handle_request_bulk_uninstall_eol(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment();
            let schedule = state.available_versions.schedule.as_ref();

            let eol_versions: Vec<String> = env
                .installed_versions
                .iter()
                .filter(|v| {
                    schedule
                        .map(|s| !s.is_active(v.version.major))
                        .unwrap_or(false)
                })
                .map(|v| v.version.to_string())
                .collect();

            if eol_versions.is_empty() {
                return Task::none();
            }

            state.modal = Some(Modal::ConfirmBulkUninstallEOL {
                versions: eol_versions,
            });
        }
        Task::none()
    }

    pub(super) fn handle_request_bulk_uninstall_major(&mut self, major: u32) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment();

            let versions: Vec<String> = env
                .installed_versions
                .iter()
                .filter(|v| v.version.major == major)
                .map(|v| v.version.to_string())
                .collect();

            if versions.is_empty() {
                return Task::none();
            }

            state.modal = Some(Modal::ConfirmBulkUninstallMajor { major, versions });
        }
        Task::none()
    }

    pub(super) fn handle_confirm_bulk_update_majors(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUpdateMajors { versions }) = state.modal.take()
        {
            for (_from, to) in versions {
                state.operation_queue.pending.push_back(QueuedOperation {
                    request: OperationRequest::Install {
                        version: to.clone(),
                    },
                });
            }
            return self.process_next_operation();
        }
        Task::none()
    }

    pub(super) fn handle_confirm_bulk_uninstall_eol(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUninstallEOL { versions }) = state.modal.take()
        {
            for version in versions {
                state.operation_queue.pending.push_back(QueuedOperation {
                    request: OperationRequest::Uninstall { version },
                });
            }
            return self.process_next_operation();
        }
        Task::none()
    }

    pub(super) fn handle_confirm_bulk_uninstall_major(&mut self, major: u32) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUninstallMajor { major: m, versions }) =
                state.modal.take()
            && m == major
        {
            for version in versions {
                state.operation_queue.pending.push_back(QueuedOperation {
                    request: OperationRequest::Uninstall { version },
                });
            }
            return self.process_next_operation();
        }
        Task::none()
    }

    pub(super) fn handle_request_bulk_uninstall_major_except_latest(
        &mut self,
        major: u32,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment();

            let mut versions_in_major: Vec<&versi_backend::InstalledVersion> = env
                .installed_versions
                .iter()
                .filter(|v| v.version.major == major)
                .collect();

            versions_in_major.sort_by(|a, b| b.version.cmp(&a.version));

            if versions_in_major.len() <= 1 {
                return Task::none();
            }

            let Some(latest) = versions_in_major.first() else {
                return Task::none();
            };
            let keeping = latest.version.to_string();

            let versions: Vec<String> = versions_in_major
                .iter()
                .skip(1)
                .map(|v| v.version.to_string())
                .collect();

            state.modal = Some(Modal::ConfirmBulkUninstallMajorExceptLatest {
                major,
                versions,
                keeping,
            });
        }
        Task::none()
    }

    pub(super) fn handle_confirm_bulk_uninstall_major_except_latest(
        &mut self,
        major: u32,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUninstallMajorExceptLatest {
                major: m, versions, ..
            }) = state.modal.take()
            && m == major
        {
            for version in versions {
                state.operation_queue.pending.push_back(QueuedOperation {
                    request: OperationRequest::Uninstall { version },
                });
            }
            return self.process_next_operation();
        }
        Task::none()
    }

    pub(super) fn process_next_operation(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.operation_queue.exclusive_op.is_some() {
                return Task::none();
            }

            let mut install_versions: Vec<String> = Vec::new();
            let mut exclusive_request: Option<OperationRequest> = None;

            while let Some(next) = state.operation_queue.pending.front() {
                match &next.request {
                    OperationRequest::Install { version } => {
                        let already_active = state.operation_queue.active_installs.iter().any(
                            |op| matches!(op, Operation::Install { version: v, .. } if v == version),
                        );
                        if !already_active && !install_versions.contains(version) {
                            install_versions.push(version.clone());
                        }
                        state.operation_queue.pending.pop_front();
                    }
                    _ => {
                        if state.operation_queue.active_installs.is_empty()
                            && install_versions.is_empty()
                            && let Some(queued) = state.operation_queue.pending.pop_front()
                        {
                            exclusive_request = Some(queued.request);
                        }
                        break;
                    }
                }
            }

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
