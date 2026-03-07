use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;
use versi_backend::{BackendUpdate, InstallProgress, NodeVersion, RemoteVersion, VersionManager};
use versi_core::{
    AppUpdate, CachedPreparedAdvisory, ReleaseSchedule, SecurityAdvisory, VersionMeta,
};
use versi_platform::EnvironmentId;

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
    pub background_preload_started: bool,
    pub available_versions: VersionCache,
    pub operation_queue: OperationQueue,
    pub bulk_run: Option<BulkRunState>,
    pub install_progress: HashMap<NodeVersion, InstallProgress>,
    pub toasts: Vec<Toast>,
    next_toast_id: usize,
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
    pub keyboard_list_mode: bool,
    pub hovered_version: Option<String>,
    pub backend_name: BackendKind,
    pub detected_backends: Vec<BackendKind>,
    pub refresh_rotation: f32,
    pub active_filters: HashSet<SearchFilter>,
    pub banner_stats: BannerStats,
    pub security_findings_by_version: HashMap<String, VersionSecurityFinding>,
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
    pub updatable_majors: usize,
    pub eol_installed: usize,
    pub vulnerable_installed: usize,
    pub vulnerable_advisory: usize,
    pub vulnerable_eol: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VersionSecurityFinding {
    pub advisory_ids: Vec<String>,
    pub is_eol: bool,
}

impl VersionSecurityFinding {
    #[must_use]
    pub fn is_vulnerable(&self) -> bool {
        self.is_eol || !self.advisory_ids.is_empty()
    }

    #[must_use]
    pub fn has_advisory_match(&self) -> bool {
        !self.advisory_ids.is_empty()
    }
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
            background_preload_started: false,
            available_versions: VersionCache::default(),
            operation_queue: OperationQueue::new(),
            bulk_run: None,
            install_progress: HashMap::new(),
            toasts: Vec::new(),
            next_toast_id: 0,
            modal: None,
            search_query: String::new(),
            backend,
            app_update: None,
            app_update_state: AppUpdateState::default(),
            app_update_check_in_flight: false,
            app_update_last_checked_at: None,
            backend_update: None,
            view: MainViewKind::default(),
            settings_state: SettingsModalState::default(),
            keyboard_list_mode: false,
            hovered_version: None,
            backend_name,
            detected_backends: Vec::new(),
            refresh_rotation: 0.0,
            active_filters: HashSet::new(),
            banner_stats: BannerStats::default(),
            security_findings_by_version: HashMap::new(),
            context_menu: None,
            cursor_position: iced::Point::ORIGIN,
        }
    }

    pub fn supports_uninstall(&self) -> bool {
        self.backend.capabilities().supports_uninstall
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

        self.recompute_security_findings();
        let vulnerable_installed_count = self
            .security_findings_by_version
            .values()
            .filter(|finding| finding.is_vulnerable())
            .count();
        let vulnerable_advisory_count = self
            .security_findings_by_version
            .values()
            .filter(|finding| finding.has_advisory_match())
            .count();
        let vulnerable_eol_count = self
            .security_findings_by_version
            .values()
            .filter(|finding| finding.is_eol)
            .count();

        self.banner_stats = BannerStats {
            updatable_majors: updatable_major_count,
            eol_installed: eol_installed_count,
            vulnerable_installed: vulnerable_installed_count,
            vulnerable_advisory: vulnerable_advisory_count,
            vulnerable_eol: vulnerable_eol_count,
        };
    }

    fn recompute_security_findings(&mut self) {
        let env = self.active_environment();
        let platform = environment_platform(&env.id);
        let prepared = self.available_versions.prepared_advisories.as_ref();
        let schedule = self.available_versions.schedule.as_ref();
        let mut findings = HashMap::new();

        for version in &env.installed_versions {
            let version_label = version.version.to_string();
            let mut advisory_ids = Vec::new();

            if let Some(prepared) = prepared {
                for (advisory_id, prepared_advisory) in prepared {
                    if prepared_advisory.affects_version_on_platform(&version_label, platform) {
                        advisory_ids.push(advisory_id.clone());
                    }
                }
            }

            advisory_ids.sort_unstable();
            let is_eol = schedule
                .is_some_and(|release_schedule| !release_schedule.is_active(version.version.major));

            let finding = VersionSecurityFinding {
                advisory_ids,
                is_eol,
            };

            if finding.is_vulnerable() {
                findings.insert(version_label, finding);
            }
        }

        self.security_findings_by_version = findings;
    }

    pub fn remove_toast(&mut self, id: usize) {
        self.toasts.retain(|t| t.id != id);
    }

    pub fn next_toast_id(&mut self) -> usize {
        let id = self.next_toast_id;
        self.next_toast_id = self.next_toast_id.wrapping_add(1);
        id
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

    pub fn should_check_for_app_updates(&self, interval: Duration) -> bool {
        if self.app_update_check_in_flight {
            return false;
        }
        self.app_update_last_checked_at
            .is_none_or(|last_checked_at| {
                Instant::now().saturating_duration_since(last_checked_at) >= interval
            })
    }

    pub fn should_check_for_security_advisories(&self, interval: Duration) -> bool {
        if self
            .available_versions
            .security_fetch
            .cancel_token
            .is_some()
        {
            return false;
        }

        self.available_versions
            .security_last_checked_at
            .is_none_or(|last_checked_at| {
                Instant::now().saturating_duration_since(last_checked_at) >= interval
            })
    }
}

/// Tracks the request lifecycle for a cancellable async fetch.
#[derive(Debug, Default)]
pub struct FetchState {
    pub request_seq: u64,
    pub cancel_token: Option<CancellationToken>,
    pub error: Option<AppError>,
}

impl FetchState {
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

#[derive(Debug, Default)]
pub struct VersionCache {
    pub versions: Arc<Vec<RemoteVersion>>,
    pub latest_by_major: HashMap<u32, NodeVersion>,
    pub lts_by_version: HashMap<NodeVersion, String>,
    pub fetched_at: Option<Instant>,
    pub loading: bool,
    pub remote: FetchState,
    pub schedule: Option<ReleaseSchedule>,
    pub schedule_fetch: FetchState,
    pub metadata: Option<Arc<HashMap<String, VersionMeta>>>,
    pub metadata_fetch: FetchState,
    pub security_advisories: Option<Arc<HashMap<String, SecurityAdvisory>>>,
    pub prepared_advisories: Option<Vec<(String, CachedPreparedAdvisory)>>,
    pub security_fetch: FetchState,
    pub security_last_checked_at: Option<Instant>,
    pub loaded_from_disk: bool,
    pub disk_cached_at: Option<DateTime<Utc>>,
    pub search_index: RemoteVersionSearchIndex,
}

impl VersionCache {
    pub fn set_versions(&mut self, versions: Vec<RemoteVersion>) {
        self.versions = Arc::new(versions);
        self.search_index = RemoteVersionSearchIndex::from_versions(&self.versions);
        self.recompute_latest_by_major();
        self.recompute_lts_by_version();
    }

    fn recompute_latest_by_major(&mut self) {
        self.latest_by_major.clear();
        self.latest_by_major.reserve(self.versions.len().min(32));
        for version in self.versions.iter() {
            self.latest_by_major
                .entry(version.version.major)
                .and_modify(|existing| {
                    if version.version > *existing {
                        *existing = version.version;
                    }
                })
                .or_insert_with(|| version.version);
        }
    }

    fn recompute_lts_by_version(&mut self) {
        self.lts_by_version.clear();
        for version in self.versions.iter() {
            if let Some(codename) = &version.lts_codename {
                self.lts_by_version
                    .insert(version.version, codename.clone());
            }
        }
    }

    pub fn set_security_advisories(&mut self, advisories: Arc<HashMap<String, SecurityAdvisory>>) {
        let prepared = advisories
            .iter()
            .map(|(id, adv)| (id.clone(), CachedPreparedAdvisory::from_advisory(adv)))
            .collect();
        self.security_advisories = Some(advisories);
        self.prepared_advisories = Some(prepared);
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

fn environment_platform(environment_id: &EnvironmentId) -> &'static str {
    match environment_id {
        EnvironmentId::Native => {
            #[cfg(target_os = "windows")]
            {
                "win32"
            }
            #[cfg(target_os = "macos")]
            {
                "darwin"
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                "linux"
            }
        }
        EnvironmentId::Wsl { .. } => "linux",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{MainState, NetworkStatus, VersionCache, VersionSecurityFinding};
    use crate::backend_kind::BackendKind;
    use crate::state::EnvironmentState;
    use versi_backend::{NodeVersion, RemoteVersion};
    use versi_core::SecurityAdvisory;
    use versi_platform::EnvironmentId;

    #[test]
    fn set_versions_recomputes_latest_major_versions() {
        let mut cache = VersionCache::default();
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
        let mut cache = VersionCache::default();
        assert!(matches!(cache.network_status(), NetworkStatus::Online));

        cache.loading = true;
        assert!(matches!(cache.network_status(), NetworkStatus::Fetching));

        cache.loading = false;
        cache.remote.error = Some(crate::error::AppError::version_fetch_failed(
            "Remote versions",
            "offline",
        ));
        assert!(matches!(cache.network_status(), NetworkStatus::Offline));

        cache.versions = Arc::new(vec![RemoteVersion {
            version: NodeVersion::new(20, 11, 0),
            lts_codename: Some("Iron".to_string()),
            is_latest: true,
        }]);
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
            disk_size: None,
        }
    }

    use crate::test_fixtures::schedule_with_eol_major;

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

        assert_eq!(state.banner_stats.updatable_majors, 1);
        assert_eq!(state.banner_stats.eol_installed, 2);
        assert_eq!(state.banner_stats.vulnerable_installed, 2);
        assert_eq!(state.banner_stats.vulnerable_advisory, 0);
        assert_eq!(state.banner_stats.vulnerable_eol, 2);
    }

    #[test]
    fn recompute_banner_stats_marks_advisory_vulnerabilities() {
        let mut state = main_state_with_native_env();
        state
            .active_environment_mut()
            .update_versions(vec![installed(NodeVersion::new(22, 21, 1), true)]);
        state
            .available_versions
            .set_security_advisories(Arc::new(HashMap::from([(
                "163".to_string(),
                SecurityAdvisory {
                    cve: vec!["CVE-2026-21637".to_string()],
                    vulnerable: "20.x || 22.x || 24.x || 25.x".to_string(),
                    patched: "^20.20.0 || ^22.22.0 || ^24.13.0 || ^25.3.0".to_string(),
                    severity: "medium".to_string(),
                    reference:
                        "https://nodejs.org/en/blog/vulnerability/december-2025-security-releases"
                            .to_string(),
                    description: "TLS callback exception handling".to_string(),
                    overview: "overview".to_string(),
                    affected_environments: vec!["all".to_string()],
                },
            )])));

        state.recompute_banner_stats();

        assert_eq!(state.banner_stats.vulnerable_installed, 1);
        assert_eq!(state.banner_stats.vulnerable_advisory, 1);
        assert_eq!(state.banner_stats.vulnerable_eol, 0);
        assert_eq!(
            state.security_findings_by_version.get("v22.21.1"),
            Some(&VersionSecurityFinding {
                advisory_ids: vec!["163".to_string()],
                is_eol: false
            })
        );
    }

    #[test]
    fn should_check_for_security_advisories_respects_interval_and_in_flight_state() {
        let mut state = main_state_with_native_env();

        assert!(state.should_check_for_security_advisories(Duration::from_secs(60)));

        state.available_versions.security_last_checked_at = Some(Instant::now());
        assert!(!state.should_check_for_security_advisories(Duration::from_secs(60)));

        state.available_versions.security_last_checked_at =
            Instant::now().checked_sub(Duration::from_secs(120));
        assert!(state.should_check_for_security_advisories(Duration::from_secs(60)));

        state.available_versions.security_fetch.cancel_token =
            Some(tokio_util::sync::CancellationToken::new());
        assert!(!state.should_check_for_security_advisories(Duration::from_secs(60)));
    }

    #[test]
    fn environment_platform_for_wsl_is_linux() {
        let platform = super::environment_platform(&EnvironmentId::Wsl {
            distro: "Ubuntu".to_string(),
            backend_path: "/home/user/.nvm/nvm.sh".to_string(),
        });

        assert_eq!(platform, "linux");
    }
}
