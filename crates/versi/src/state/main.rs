use std::time::Instant;

use versi_core::{AppUpdate, BackendUpdate, ReleaseSchedule, RemoteVersion, VersionManager};

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
    pub backend_update: Option<BackendUpdate>,
    pub view: MainViewKind,
    pub settings_state: SettingsModalState,
    pub hovered_version: Option<String>,
    pub backend_name: &'static str,
    pub refresh_rotation: f32,
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
            backend_update: None,
            view: MainViewKind::default(),
            settings_state: SettingsModalState::new(),
            hovered_version: None,
            backend_name,
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
        }
    }

    pub fn network_status(&self) -> NetworkStatus {
        if self.loading {
            return NetworkStatus::Fetching;
        }
        if let Some(err) = &self.error {
            if self.versions.is_empty() {
                return NetworkStatus::Offline(err.clone());
            }
            return NetworkStatus::Stale(err.clone());
        }
        NetworkStatus::Online
    }
}

#[allow(dead_code)]
pub enum NetworkStatus {
    Online,
    Fetching,
    Offline(String),
    Stale(String),
}
