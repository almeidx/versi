use std::collections::HashMap;
use std::time::Instant;

use chrono::{DateTime, Utc};
use versi_backend::{BackendUpdate, RemoteVersion, VersionManager};
use versi_core::{AppUpdate, ReleaseSchedule};

use super::{EnvironmentState, MainViewKind, Modal, OperationQueue, SettingsModalState, Toast};

pub struct MainState {
    pub environments: Vec<EnvironmentState>,
    pub active_environment_idx: usize,
    pub available_versions: VersionCache,
    pub operation_queue: OperationQueue,
    pub toasts: Vec<Toast>,
    pub modal: Option<Modal>,
    pub search_query: String,
    pub backend: Box<dyn VersionManager>,
    pub app_update: Option<AppUpdate>,
    pub app_update_state: AppUpdateState,
    pub backend_update: Option<BackendUpdate>,
    pub view: MainViewKind,
    pub settings_state: SettingsModalState,
    pub hovered_version: Option<String>,
    pub backend_name: &'static str,
    pub detected_backends: Vec<&'static str>,
    pub refresh_rotation: f32,
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
    Failed(String),
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
            .finish()
    }
}

impl MainState {
    pub fn new_with_environments(
        backend: Box<dyn VersionManager>,
        environments: Vec<EnvironmentState>,
        backend_name: &'static str,
    ) -> Self {
        Self {
            environments,
            active_environment_idx: 0,
            available_versions: VersionCache::new(),
            operation_queue: OperationQueue::new(),
            toasts: Vec::new(),
            modal: None,
            search_query: String::new(),
            backend,
            app_update: None,
            app_update_state: AppUpdateState::default(),
            backend_update: None,
            view: MainViewKind::default(),
            settings_state: SettingsModalState::new(),
            hovered_version: None,
            backend_name,
            detected_backends: Vec::new(),
            refresh_rotation: 0.0,
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

    pub fn remove_toast(&mut self, id: usize) {
        self.toasts.retain(|t| t.id != id);
    }

    pub fn next_toast_id(&self) -> usize {
        self.toasts.iter().map(|t| t.id).max().unwrap_or(0) + 1
    }

    pub fn navigable_versions(&self) -> Vec<String> {
        let env = self.active_environment();
        let mut result = Vec::new();

        if self.search_query.is_empty() {
            for group in &env.version_groups {
                if group.is_expanded {
                    for v in &group.versions {
                        result.push(v.version.to_string());
                    }
                }
            }
        } else {
            let query = &self.search_query;
            let query_lower = query.to_lowercase();

            let mut filtered: Vec<&RemoteVersion> = self
                .available_versions
                .versions
                .iter()
                .filter(|v| {
                    let version_str = v.version.to_string();
                    if query_lower == "lts" {
                        return v.lts_codename.is_some();
                    }
                    version_str.contains(query.as_str())
                        || v.lts_codename
                            .as_ref()
                            .map(|c| c.to_lowercase().contains(&query_lower))
                            .unwrap_or(false)
                })
                .collect();

            filtered.sort_by(|a, b| b.version.cmp(&a.version));

            let mut latest_by_minor: HashMap<(u32, u32), &RemoteVersion> = HashMap::new();
            for v in &filtered {
                let key = (v.version.major, v.version.minor);
                latest_by_minor
                    .entry(key)
                    .and_modify(|existing| {
                        if v.version.patch > existing.version.patch {
                            *existing = v;
                        }
                    })
                    .or_insert(v);
            }

            let mut available: Vec<&RemoteVersion> = latest_by_minor.into_values().collect();
            available.sort_by(|a, b| b.version.cmp(&a.version));
            available.truncate(20);

            for v in available {
                result.push(v.version.to_string());
            }
        }

        result
    }

    pub fn is_version_installed(&self, version_str: &str) -> bool {
        self.active_environment()
            .installed_versions
            .iter()
            .any(|v| v.version.to_string() == version_str)
    }
}

#[derive(Debug)]
pub struct VersionCache {
    pub versions: Vec<RemoteVersion>,
    pub fetched_at: Option<Instant>,
    pub loading: bool,
    pub error: Option<String>,
    pub schedule: Option<ReleaseSchedule>,
    pub schedule_error: Option<String>,
    pub loaded_from_disk: bool,
    pub disk_cached_at: Option<DateTime<Utc>>,
}

impl VersionCache {
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
            fetched_at: None,
            loading: false,
            error: None,
            schedule: None,
            schedule_error: None,
            loaded_from_disk: false,
            disk_cached_at: None,
        }
    }

    pub fn network_status(&self) -> NetworkStatus {
        if self.loading {
            return NetworkStatus::Fetching;
        }
        if self.error.is_some() {
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
