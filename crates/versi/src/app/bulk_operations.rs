use iced::Task;
use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion};

use crate::message::Message;
use crate::state::{
    AppState, BulkItemStatus, BulkRunAction, BulkRunItem, BulkRunKind, BulkRunState, MainState,
    Modal, Operation,
};

use super::Versi;

fn unique_versions_in_order(versions: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();

    for version in versions {
        if seen.insert(version.clone()) {
            deduped.push(version);
        }
    }

    deduped
}

fn bulk_run_items(versions: &[String], action: BulkRunAction) -> Vec<BulkRunItem> {
    versions
        .iter()
        .map(|version| BulkRunItem {
            version: version.clone(),
            action,
            status: BulkItemStatus::Pending,
        })
        .collect()
}

fn enqueue_bulk_operations(state: &mut MainState, versions: Vec<String>, action: BulkRunAction) {
    for version in versions {
        let operation = match action {
            BulkRunAction::Install => Operation::Install { version },
            BulkRunAction::Uninstall => Operation::Uninstall { version },
        };
        state.operation_queue.enqueue(operation);
    }
}

fn start_bulk_run(
    state: &mut MainState,
    kind: BulkRunKind,
    action: BulkRunAction,
    versions: Vec<String>,
) -> bool {
    if state.bulk_run.is_some() {
        return false;
    }

    let targets = unique_versions_in_order(versions);
    if targets.is_empty() {
        return false;
    }

    state.bulk_run = Some(BulkRunState::new(kind, bulk_run_items(&targets, action)));
    enqueue_bulk_operations(state, targets, action);
    true
}

fn latest_by_major<T>(
    items: &[T],
    version_of: impl Fn(&T) -> &NodeVersion,
) -> std::collections::HashMap<u32, NodeVersion> {
    let mut latest = std::collections::HashMap::new();
    for item in items {
        let v = version_of(item);
        latest
            .entry(v.major)
            .and_modify(|existing: &mut NodeVersion| {
                if *v > *existing {
                    *existing = v.clone();
                }
            })
            .or_insert_with(|| v.clone());
    }
    latest
}

fn compute_major_updates(
    installed: &[InstalledVersion],
    remote: &[RemoteVersion],
) -> Vec<(String, String)> {
    let latest_remote = latest_by_major(remote, |v| &v.version);
    let latest_installed = latest_by_major(installed, |v| &v.version);

    latest_installed
        .iter()
        .filter_map(|(major, installed_version)| {
            latest_remote.get(major).and_then(|latest_version| {
                if latest_version > installed_version {
                    Some((installed_version.to_string(), latest_version.to_string()))
                } else {
                    None
                }
            })
        })
        .collect()
}

fn versions_for_major(installed: &[InstalledVersion], major: u32) -> Vec<String> {
    installed
        .iter()
        .filter(|version| version.version.major == major)
        .map(|version| version.version.to_string())
        .collect()
}

fn versions_to_uninstall_except_latest(
    installed: &[InstalledVersion],
    major: u32,
) -> Option<(Vec<String>, String)> {
    let mut versions_in_major: Vec<&InstalledVersion> = installed
        .iter()
        .filter(|version| version.version.major == major)
        .collect();
    versions_in_major.sort_by(|a, b| b.version.cmp(&a.version));

    if versions_in_major.len() <= 1 {
        return None;
    }

    let keeping = versions_in_major.first()?.version.to_string();
    let removing = versions_in_major
        .iter()
        .skip(1)
        .map(|version| version.version.to_string())
        .collect();
    Some((removing, keeping))
}

impl Versi {
    pub(super) fn handle_request_bulk_update_majors(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.bulk_run.is_some() {
                return Task::none();
            }

            let env = state.active_environment();
            let remote = &state.available_versions.versions;
            let versions_to_update = compute_major_updates(&env.installed_versions, remote);

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
            if state.bulk_run.is_some() {
                return Task::none();
            }

            let env = state.active_environment();
            let schedule = state.available_versions.schedule.as_ref();

            let eol_versions: Vec<String> = env
                .installed_versions
                .iter()
                .filter(|v| schedule.is_some_and(|s| !s.is_active(v.version.major)))
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
            if state.bulk_run.is_some() {
                return Task::none();
            }

            let env = state.active_environment();
            let versions = versions_for_major(&env.installed_versions, major);

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
            let started = start_bulk_run(
                state,
                BulkRunKind::UpdateMajors,
                BulkRunAction::Install,
                versions.into_iter().map(|(_from, to)| to).collect(),
            );
            if started {
                return self.process_next_operation();
            }
        }
        Task::none()
    }

    pub(super) fn handle_confirm_bulk_uninstall_eol(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUninstallEOL { versions }) = state.modal.take()
        {
            let started = start_bulk_run(
                state,
                BulkRunKind::UninstallEol,
                BulkRunAction::Uninstall,
                versions,
            );
            if started {
                return self.process_next_operation();
            }
        }
        Task::none()
    }

    pub(super) fn handle_confirm_bulk_uninstall_major(&mut self, major: u32) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUninstallMajor { major: m, versions }) =
                state.modal.take()
            && m == major
        {
            let started = start_bulk_run(
                state,
                BulkRunKind::UninstallMajor,
                BulkRunAction::Uninstall,
                versions,
            );
            if started {
                return self.process_next_operation();
            }
        }
        Task::none()
    }

    pub(super) fn handle_request_bulk_uninstall_major_except_latest(
        &mut self,
        major: u32,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.bulk_run.is_some() {
                return Task::none();
            }

            let env = state.active_environment();

            let Some((versions, keeping)) =
                versions_to_uninstall_except_latest(&env.installed_versions, major)
            else {
                return Task::none();
            };

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
            let started = start_bulk_run(
                state,
                BulkRunKind::UninstallMajorExceptLatest,
                BulkRunAction::Uninstall,
                versions,
            );
            if started {
                return self.process_next_operation();
            }
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use versi_backend::{InstalledVersion, RemoteVersion};

    use super::{compute_major_updates, versions_for_major, versions_to_uninstall_except_latest};

    fn installed(version: &str) -> InstalledVersion {
        InstalledVersion {
            version: version.parse().expect("test version should parse"),
            is_default: false,
            lts_codename: None,
            install_date: Some(Utc::now()),
            disk_size: None,
        }
    }

    fn remote(version: &str) -> RemoteVersion {
        RemoteVersion {
            version: version.parse().expect("test version should parse"),
            lts_codename: None,
            is_latest: false,
        }
    }

    #[test]
    fn compute_major_updates_returns_only_outdated_installed_majors() {
        let installed = vec![
            installed("v22.3.0"),
            installed("v20.11.1"),
            installed("v18.19.0"),
        ];
        let remote = vec![remote("v22.8.0"), remote("v20.11.1"), remote("v18.20.0")];

        let mut updates = compute_major_updates(&installed, &remote);
        updates.sort();

        assert_eq!(
            updates,
            vec![
                ("v18.19.0".to_string(), "v18.20.0".to_string()),
                ("v22.3.0".to_string(), "v22.8.0".to_string()),
            ]
        );
    }

    #[test]
    fn versions_for_major_filters_to_matching_major_only() {
        let installed = vec![
            installed("v22.3.0"),
            installed("v20.11.1"),
            installed("v22.8.0"),
        ];

        let versions = versions_for_major(&installed, 22);

        assert_eq!(versions, vec!["v22.3.0".to_string(), "v22.8.0".to_string()]);
    }

    #[test]
    fn uninstall_except_latest_returns_sorted_removals_and_kept_version() {
        let installed = vec![
            installed("v22.1.0"),
            installed("v22.9.0"),
            installed("v22.7.0"),
        ];

        let (remove, keep) = versions_to_uninstall_except_latest(&installed, 22)
            .expect("major with more than one version should produce removals");

        assert_eq!(keep, "v22.9.0");
        assert_eq!(remove, vec!["v22.7.0".to_string(), "v22.1.0".to_string()]);
    }

    #[test]
    fn uninstall_except_latest_returns_none_for_single_version() {
        let installed = vec![installed("v22.9.0")];
        assert!(versions_to_uninstall_except_latest(&installed, 22).is_none());
    }
}
