use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;
use versi_backend::{BackendUpdate, InstallProgress, NodeVersion, RemoteVersion, VersionManager};
use versi_core::{AppUpdate, ReleaseSchedule, VersionMeta};

use crate::backend_kind::BackendKind;
use crate::error::AppError;
use crate::version_query::{RemoteVersionSearchIndex, search_available_versions_with_index};

use super::{
    BulkRunState, ContextMenu, EnvironmentState, MainViewKind, Modal, OperationQueue,
    SettingsModalState, Toast,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchFilter {
    Lts,
    Installed,
    NotInstalled,
    Eol,
    Active,
}

pub struct MainState {
    pub environments: Vec<EnvironmentState>,
    pub active_environment_idx: usize,
    pub available_versions: VersionCache,
    pub operation_queue: OperationQueue,
    pub bulk_run: Option<BulkRunState>,
    pub install_progress: HashMap<String, InstallProgress>,
    pub toasts: Vec<Toast>,
    pub modal: Option<Modal>,
    pub search_query: String,
    pub backend: Arc<dyn VersionManager>,
    pub app_update: Option<AppUpdate>,
    pub app_update_state: AppUpdateState,
    pub app_update_check_in_flight: bool,
    pub app_update_last_checked_at: Option<Instant>,
    pub backend_update: Option<BackendUpdate>,
    pub view: MainViewKind,
    pub settings_state: SettingsModalState,
    pub hovered_version: Option<String>,
    pub backend_name: BackendKind,
    pub detected_backends: Vec<BackendKind>,
    pub refresh_rotation: f32,
    pub active_filters: HashSet<SearchFilter>,
    pub banner_stats: BannerStats,
    pub context_menu: Option<ContextMenu>,
    pub cursor_position: iced::Point,
}

#[derive(Debug, Clone, Default)]
pub enum AppUpdateState {
    #[default]
    Idle,
    Downloading {
        downloaded: u64,
        total: u64,
    },
    Extracting,
    Applying,
    RestartRequired,
    Failed(AppError),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BannerStats {
    pub updatable_major_count: usize,
    pub eol_installed_count: usize,
}

impl std::fmt::Debug for MainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainState")
            .field("environments", &self.environments)
            .field("active_environment_idx", &self.active_environment_idx)
            .field("available_versions", &self.available_versions)
            .field("operation_queue", &self.operation_queue)
            .field("toasts", &self.toasts)
            .field("modal", &self.modal)
            .field("search_query", &self.search_query)
            .field("backend", &self.backend.name())
            .field("app_update", &self.app_update)
            .field("backend_update", &self.backend_update)
            .field("view", &self.view)
            .field("hovered_version", &self.hovered_version)
            .finish_non_exhaustive()
    }
}

impl MainState {
    pub fn new_with_environments(
        backend: Arc<dyn VersionManager>,
        environments: Vec<EnvironmentState>,
        backend_name: BackendKind,
    ) -> Self {
        Self {
            environments,
            active_environment_idx: 0,
            available_versions: VersionCache::new(),
            operation_queue: OperationQueue::new(),
            bulk_run: None,
            install_progress: HashMap::new(),
            toasts: Vec::new(),
            modal: None,
            search_query: String::new(),
            backend,
            app_update: None,
            app_update_state: AppUpdateState::default(),
            app_update_check_in_flight: false,
            app_update_last_checked_at: None,
            backend_update: None,
            view: MainViewKind::default(),
            settings_state: SettingsModalState::new(),
            hovered_version: None,
            backend_name,
            detected_backends: Vec::new(),
            refresh_rotation: 0.0,
            active_filters: HashSet::new(),
            banner_stats: BannerStats::default(),
            context_menu: None,
            cursor_position: iced::Point::ORIGIN,
        }
    }

    pub fn active_environment(&self) -> &EnvironmentState {
        &self.environments[self.active_environment_idx]
    }

    pub fn active_environment_mut(&mut self) -> &mut EnvironmentState {
        &mut self.environments[self.active_environment_idx]
    }

    pub fn add_toast(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    pub fn recompute_banner_stats(&mut self) {
        let env = &self.environments[self.active_environment_idx];
        let updatable_major_count = env
            .version_groups
            .iter()
            .filter(|group| {
                let installed_latest = group.versions.iter().map(|v| &v.version).max();
                self.available_versions
                    .latest_by_major
                    .get(&group.major)
                    .is_some_and(|latest| {
                        installed_latest.is_some_and(|installed| latest > installed)
                    })
            })
            .count();

        let eol_installed_count = self
            .available_versions
            .schedule
            .as_ref()
            .map_or(0, |schedule| {
                env.version_groups
                    .iter()
                    .filter(|group| !schedule.is_active(group.major))
                    .map(|group| group.versions.len())
                    .sum::<usize>()
            });

        self.banner_stats = BannerStats {
            updatable_major_count,
            eol_installed_count,
        };
    }

    pub fn remove_toast(&mut self, id: usize) {
        self.toasts.retain(|t| t.id != id);
    }

    pub fn next_toast_id(&self) -> usize {
        self.toasts.iter().map(|t| t.id).max().unwrap_or(0) + 1
    }

    pub fn navigable_versions(&self, search_results_limit: usize) -> Vec<String> {
        let env = self.active_environment();
        let mut result = Vec::new();
        let mut version_text = String::with_capacity(16);

        if self.search_query.is_empty() {
            for group in &env.version_groups {
                if group.is_expanded {
                    for v in &group.versions {
                        v.version.write_prefixed_into(&mut version_text);
                        result.push(version_text.clone());
                    }
                }
            }
        } else {
            let search = search_available_versions_with_index(
                &self.available_versions.versions,
                Some(&self.available_versions.search_index),
                &self.search_query,
                search_results_limit,
                &self.active_filters,
                &env.installed_set,
                self.available_versions.schedule.as_ref(),
            );

            for v in search.versions {
                v.version.write_prefixed_into(&mut version_text);
                result.push(version_text.clone());
            }
        }

        result
    }

    pub fn is_version_installed(&self, version_str: &str) -> bool {
        version_str
            .parse()
            .ok()
            .is_some_and(|version| self.active_environment().installed_set.contains(&version))
    }

    pub fn should_check_for_app_updates(&self, interval: Duration) -> bool {
        if self.app_update_check_in_flight {
            return false;
        }
        self.app_update_last_checked_at
            .is_none_or(|last_checked_at| {
                Instant::now().saturating_duration_since(last_checked_at) >= interval
            })
    }
}

/// Tracks the request lifecycle for a cancellable async fetch.
#[derive(Debug)]
pub struct FetchState {
    pub request_seq: u64,
    pub cancel_token: Option<CancellationToken>,
    pub error: Option<AppError>,
}

impl FetchState {
    pub fn new() -> Self {
        Self {
            request_seq: 0,
            cancel_token: None,
            error: None,
        }
    }

    /// Cancel any in-flight request and start a new one.
    /// Returns `(cancel_token, request_seq)` for the new request.
    pub fn start(&mut self) -> (CancellationToken, u64) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
        self.request_seq = self.request_seq.wrapping_add(1);
        let token = CancellationToken::new();
        self.cancel_token = Some(token.clone());
        (token, self.request_seq)
    }

    /// Check whether a response is still current. If so, clear the cancel
    /// token and return `true`; otherwise return `false` (stale).
    pub fn accept(&mut self, request_seq: u64) -> bool {
        if request_seq != self.request_seq {
            return false;
        }
        self.cancel_token = None;
        true
    }
}

#[derive(Debug)]
pub struct VersionCache {
    pub versions: Vec<RemoteVersion>,
    pub latest_by_major: HashMap<u32, NodeVersion>,
    pub fetched_at: Option<Instant>,
    pub loading: bool,
    pub remote: FetchState,
    pub schedule: Option<ReleaseSchedule>,
    pub schedule_fetch: FetchState,
    pub metadata: Option<HashMap<String, VersionMeta>>,
    pub metadata_fetch: FetchState,
    pub loaded_from_disk: bool,
    pub disk_cached_at: Option<DateTime<Utc>>,
    pub search_index: RemoteVersionSearchIndex,
}

impl VersionCache {
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
            latest_by_major: HashMap::new(),
            fetched_at: None,
            loading: false,
            remote: FetchState::new(),
            schedule: None,
            schedule_fetch: FetchState::new(),
            metadata: None,
            metadata_fetch: FetchState::new(),
            loaded_from_disk: false,
            disk_cached_at: None,
            search_index: RemoteVersionSearchIndex::default(),
        }
    }

    pub fn set_versions(&mut self, versions: Vec<RemoteVersion>) {
        self.versions = versions;
        self.search_index = RemoteVersionSearchIndex::from_versions(&self.versions);
        self.recompute_latest_by_major();
    }

    fn recompute_latest_by_major(&mut self) {
        self.latest_by_major.clear();
        self.latest_by_major.reserve(self.versions.len().min(32));
        for version in &self.versions {
            self.latest_by_major
                .entry(version.version.major)
                .and_modify(|existing| {
                    if version.version > *existing {
                        *existing = version.version.clone();
                    }
                })
                .or_insert_with(|| version.version.clone());
        }
    }

    pub fn network_status(&self) -> NetworkStatus {
        if self.loading {
            return NetworkStatus::Fetching;
        }
        if self.remote.error.is_some() {
            if self.versions.is_empty() {
                return NetworkStatus::Offline;
            }
            return NetworkStatus::Stale;
        }
        NetworkStatus::Online
    }
}

pub enum NetworkStatus {
    Online,
    Fetching,
    Offline,
    Stale,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{MainState, NetworkStatus, SearchFilter, VersionCache};
    use crate::backend_kind::BackendKind;
    use crate::state::EnvironmentState;
    use versi_backend::{NodeVersion, RemoteVersion};
    use versi_platform::EnvironmentId;

    #[test]
    fn set_versions_recomputes_latest_major_versions() {
        let mut cache = VersionCache::new();
        cache.set_versions(vec![
            RemoteVersion {
                version: NodeVersion::new(20, 10, 0),
                lts_codename: Some("Iron".to_string()),
                is_latest: false,
            },
            RemoteVersion {
                version: NodeVersion::new(20, 11, 0),
                lts_codename: Some("Iron".to_string()),
                is_latest: true,
            },
            RemoteVersion {
                version: NodeVersion::new(22, 1, 0),
                lts_codename: None,
                is_latest: true,
            },
            RemoteVersion {
                version: NodeVersion::new(22, 0, 1),
                lts_codename: None,
                is_latest: false,
            },
        ]);

        assert_eq!(
            cache.latest_by_major.get(&20),
            Some(&NodeVersion::new(20, 11, 0))
        );
        assert_eq!(
            cache.latest_by_major.get(&22),
            Some(&NodeVersion::new(22, 1, 0))
        );
    }

    #[test]
    fn network_status_reports_expected_state() {
        let mut cache = VersionCache::new();
        assert!(matches!(cache.network_status(), NetworkStatus::Online));

        cache.loading = true;
        assert!(matches!(cache.network_status(), NetworkStatus::Fetching));

        cache.loading = false;
        cache.remote.error = Some(crate::error::AppError::version_fetch_failed(
            "Remote versions",
            "offline",
        ));
        assert!(matches!(cache.network_status(), NetworkStatus::Offline));

        cache.versions = vec![RemoteVersion {
            version: NodeVersion::new(20, 11, 0),
            lts_codename: Some("Iron".to_string()),
            is_latest: true,
        }];
        assert!(matches!(cache.network_status(), NetworkStatus::Stale));
    }

    fn main_state_with_native_env() -> MainState {
        let provider: std::sync::Arc<dyn versi_backend::BackendProvider> =
            std::sync::Arc::new(versi_fnm::FnmProvider::new());
        let backend = provider.create_manager(&versi_backend::BackendDetection {
            found: true,
            path: Some(PathBuf::from("fnm")),
            version: None,
            in_path: true,
            data_dir: None,
        });
        let mut env = EnvironmentState::new(EnvironmentId::Native, BackendKind::Fnm, None);
        env.loading = false;
        MainState::new_with_environments(backend, vec![env], BackendKind::Fnm)
    }

    fn remote(version: NodeVersion, lts: Option<&str>) -> RemoteVersion {
        RemoteVersion {
            version,
            lts_codename: lts.map(str::to_string),
            is_latest: false,
        }
    }

    fn installed(version: NodeVersion, is_default: bool) -> versi_backend::InstalledVersion {
        versi_backend::InstalledVersion {
            version,
            is_default,
            lts_codename: None,
            install_date: None,
            disk_size: None,
        }
    }

    fn schedule_with_eol_major(eol_major: u32) -> versi_core::ReleaseSchedule {
        serde_json::from_value(serde_json::json!({
            "versions": {
                format!("{eol_major}"): {
                    "start": "2020-01-01",
                    "end": "2021-01-01"
                },
                "22": {
                    "start": "2024-04-23",
                    "lts": "2024-10-29",
                    "maintenance": "2026-10-20",
                    "end": "2027-04-30",
                    "codename": "Jod"
                }
            }
        }))
        .expect("schedule fixture should deserialize")
    }

    #[test]
    fn navigable_versions_uses_expanded_groups_without_search() {
        let mut state = main_state_with_native_env();
        state.active_environment_mut().update_versions(vec![
            installed(NodeVersion::new(22, 3, 1), true),
            installed(NodeVersion::new(20, 11, 0), false),
        ]);
        state
            .active_environment_mut()
            .version_groups
            .iter_mut()
            .for_each(|g| g.is_expanded = g.major == 22);

        let navigable = state.navigable_versions(10);

        assert_eq!(navigable, vec!["v22.3.1".to_string()]);
    }

    #[test]
    fn navigable_versions_resolves_alias_queries() {
        let mut state = main_state_with_native_env();
        state.available_versions.set_versions(vec![
            remote(NodeVersion::new(24, 1, 0), None),
            remote(NodeVersion::new(22, 11, 0), Some("Jod")),
            remote(NodeVersion::new(20, 12, 0), Some("Iron")),
        ]);

        state.search_query = "latest".to_string();
        assert_eq!(state.navigable_versions(10), vec!["v24.1.0".to_string()]);

        state.search_query = "lts/iron".to_string();
        assert_eq!(state.navigable_versions(10), vec!["v20.12.0".to_string()]);
    }

    #[test]
    fn is_version_installed_checks_active_environment_versions() {
        let mut state = main_state_with_native_env();
        state.active_environment_mut().installed_versions = vec![versi_backend::InstalledVersion {
            version: NodeVersion::new(20, 11, 0),
            is_default: true,
            lts_codename: Some("Iron".to_string()),
            install_date: None,
            disk_size: None,
        }];
        state
            .active_environment_mut()
            .installed_set
            .insert(NodeVersion::new(20, 11, 0));
        state.active_filters.insert(SearchFilter::Lts);

        assert!(state.is_version_installed("v20.11.0"));
        assert!(!state.is_version_installed("v18.19.1"));
    }

    #[test]
    fn recompute_banner_stats_tracks_updates_and_eol_counts() {
        let mut state = main_state_with_native_env();
        state.active_environment_mut().update_versions(vec![
            installed(NodeVersion::new(22, 1, 0), false),
            installed(NodeVersion::new(20, 11, 0), false),
            installed(NodeVersion::new(20, 10, 0), false),
        ]);
        state.available_versions.latest_by_major = std::collections::HashMap::from([
            (22, NodeVersion::new(22, 3, 0)),
            (20, NodeVersion::new(20, 11, 0)),
        ]);
        state.available_versions.schedule = Some(schedule_with_eol_major(20));

        state.recompute_banner_stats();

        assert_eq!(state.banner_stats.updatable_major_count, 1);
        assert_eq!(state.banner_stats.eol_installed_count, 2);
    }
}
