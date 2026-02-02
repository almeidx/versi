use iced::Task;

use crate::message::Message;
use crate::state::{AppState, Modal, OperationRequest};

use super::Versi;

impl Versi {
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
                state
                    .operation_queue
                    .enqueue(OperationRequest::Install { version: to });
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
                state
                    .operation_queue
                    .enqueue(OperationRequest::Uninstall { version });
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
                state
                    .operation_queue
                    .enqueue(OperationRequest::Uninstall { version });
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
                state
                    .operation_queue
                    .enqueue(OperationRequest::Uninstall { version });
            }
            return self.process_next_operation();
        }
        Task::none()
    }
}
