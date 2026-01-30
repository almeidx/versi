use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;
use versi_core::{
    AppUpdate, FnmUpdate, InstallProgress, InstalledVersion, NodeVersion, ReleaseSchedule,
    RemoteVersion, VersionGroup, VersionManager,
};
use versi_platform::EnvironmentId;
use versi_shell::ShellType;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum MainViewKind {
    #[default]
    Versions,
    Settings,
    About,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum AppState {
    Loading,
    Onboarding(OnboardingState),
    Main(MainState),
}

#[derive(Debug)]
pub struct OnboardingState {
    pub step: OnboardingStep,
    pub fnm_installing: bool,
    pub install_error: Option<String>,
    pub detected_shells: Vec<ShellConfigStatus>,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Welcome,
            fnm_installing: false,
            install_error: None,
            detected_shells: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    Welcome,
    InstallFnm,
    ConfigureShell,
}

#[derive(Debug, Clone)]
pub struct ShellConfigStatus {
    pub shell_type: ShellType,
    pub shell_name: String,
    pub configured: bool,
    pub config_path: Option<PathBuf>,
    pub configuring: bool,
    pub error: Option<String>,
}

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
    pub fnm_update: Option<FnmUpdate>,
    pub view: MainViewKind,
    pub settings_state: SettingsModalState,
    pub hovered_version: Option<String>,
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
            .field("fnm_update", &self.fnm_update)
            .field("view", &self.view)
            .field("hovered_version", &self.hovered_version)
            .finish()
    }
}

impl MainState {
    pub fn new(backend: Box<dyn VersionManager>, fnm_version: Option<String>) -> Self {
        Self {
            environments: vec![EnvironmentState::new(EnvironmentId::Native, fnm_version)],
            active_environment_idx: 0,
            available_versions: VersionCache::new(),
            operation_queue: OperationQueue::new(),
            toasts: Vec::new(),
            modal: None,
            search_query: String::new(),
            backend,
            app_update: None,
            fnm_update: None,
            view: MainViewKind::default(),
            settings_state: SettingsModalState::new(),
            hovered_version: None,
        }
    }

    pub fn new_with_environments(
        backend: Box<dyn VersionManager>,
        environments: Vec<EnvironmentState>,
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
            fnm_update: None,
            view: MainViewKind::default(),
            settings_state: SettingsModalState::new(),
            hovered_version: None,
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
pub struct EnvironmentState {
    pub id: EnvironmentId,
    pub name: String,
    pub installed_versions: Vec<InstalledVersion>,
    pub version_groups: Vec<VersionGroup>,
    pub default_version: Option<NodeVersion>,
    pub fnm_version: Option<String>,
    pub loading: bool,
    pub error: Option<String>,
    pub available: bool,
}

impl EnvironmentState {
    pub fn new(id: EnvironmentId, fnm_version: Option<String>) -> Self {
        let name = id.display_name();
        Self {
            id,
            name,
            installed_versions: Vec::new(),
            version_groups: Vec::new(),
            default_version: None,
            fnm_version,
            loading: true,
            error: None,
            available: true,
        }
    }

    pub fn unavailable(id: EnvironmentId, reason: &str) -> Self {
        let name = id.display_name();
        Self {
            id,
            name,
            installed_versions: Vec::new(),
            version_groups: Vec::new(),
            default_version: None,
            fnm_version: None,
            loading: false,
            error: Some(reason.to_string()),
            available: false,
        }
    }

    pub fn update_versions(&mut self, versions: Vec<InstalledVersion>) {
        self.default_version = versions
            .iter()
            .find(|v| v.is_default)
            .map(|v| v.version.clone());
        self.version_groups = VersionGroup::from_versions(versions.clone());
        self.installed_versions = versions;
        self.loading = false;
        self.error = None;
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

#[allow(dead_code)]
pub enum NetworkStatus {
    Online,
    Fetching,
    Offline(String),
    Stale(String),
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

#[derive(Debug, Clone)]
pub enum Operation {
    Install {
        version: String,
        progress: InstallProgress,
    },
    Uninstall {
        version: String,
    },
    SetDefault {
        version: String,
    },
}

#[derive(Debug, Clone)]
pub enum OperationRequest {
    Install { version: String },
    Uninstall { version: String },
    SetDefault { version: String },
}

impl OperationRequest {
    pub fn version(&self) -> &str {
        match self {
            Self::Install { version } => version,
            Self::Uninstall { version } => version,
            Self::SetDefault { version } => version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueuedOperation {
    pub request: OperationRequest,
    #[allow(dead_code)]
    pub queued_at: Instant,
}

#[derive(Clone)]
pub struct OperationQueue {
    pub active_installs: Vec<Operation>,
    pub exclusive_op: Option<Operation>,
    pub pending: VecDeque<QueuedOperation>,
}

impl std::fmt::Debug for OperationQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationQueue")
            .field("active_installs", &self.active_installs.len())
            .field("exclusive_op", &self.exclusive_op)
            .field("pending", &self.pending.len())
            .finish()
    }
}

impl Default for OperationQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationQueue {
    pub fn new() -> Self {
        Self {
            active_installs: Vec::new(),
            exclusive_op: None,
            pending: VecDeque::new(),
        }
    }

    pub fn is_busy_for_install(&self) -> bool {
        self.exclusive_op.is_some()
    }

    pub fn is_busy_for_exclusive(&self) -> bool {
        !self.active_installs.is_empty() || self.exclusive_op.is_some()
    }

    #[allow(dead_code)]
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    #[allow(dead_code)]
    pub fn queue_count(&self) -> usize {
        self.pending.len()
    }

    pub fn has_pending_for_version(&self, version: &str) -> bool {
        self.pending
            .iter()
            .any(|op| op.request.version() == version)
    }

    pub fn is_current_version(&self, version: &str) -> bool {
        let in_installs = self.active_installs.iter().any(|op| match op {
            Operation::Install { version: v, .. } => v == version,
            _ => false,
        });
        if in_installs {
            return true;
        }
        self.exclusive_op
            .as_ref()
            .map(|op| match op {
                Operation::Install { version: v, .. } => v == version,
                Operation::Uninstall { version: v } => v == version,
                Operation::SetDefault { version: v } => v == version,
            })
            .unwrap_or(false)
    }

    pub fn active_operation_for(&self, version: &str) -> Option<&Operation> {
        if let Some(op) = self
            .active_installs
            .iter()
            .find(|op| matches!(op, Operation::Install { version: v, .. } if v == version))
        {
            return Some(op);
        }
        self.exclusive_op.as_ref().filter(|op| match op {
            Operation::Install { version: v, .. } => v == version,
            Operation::Uninstall { version: v } => v == version,
            Operation::SetDefault { version: v } => v == version,
        })
    }

    pub fn remove_completed_install(&mut self, version: &str) {
        self.active_installs.retain(|op| match op {
            Operation::Install { version: v, .. } => v != version,
            _ => true,
        });
    }

    pub fn update_install_progress(&mut self, version: &str, progress: InstallProgress) {
        if let Some(Operation::Install {
            progress: op_progress,
            ..
        }) = self
            .active_installs
            .iter_mut()
            .find(|op| matches!(op, Operation::Install { version: v, .. } if v == version))
        {
            *op_progress = progress;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: usize,
    pub message: String,
    pub created_at: Instant,
}

impl Toast {
    pub fn error(id: usize, message: String) -> Self {
        Self {
            id,
            message,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > 5
    }
}

#[derive(Debug, Clone)]
pub enum Modal {
    ConfirmBulkUpdateMajors {
        versions: Vec<(String, String)>,
    },
    ConfirmBulkUninstallEOL {
        versions: Vec<String>,
    },
    ConfirmBulkUninstallMajor {
        major: u32,
        versions: Vec<String>,
    },
    ConfirmBulkUninstallMajorExceptLatest {
        major: u32,
        versions: Vec<String>,
        keeping: String,
    },
}

#[derive(Debug, Clone)]
pub struct SettingsModalState {
    pub shell_statuses: Vec<ShellSetupStatus>,
    pub checking_shells: bool,
    pub log_file_size: Option<u64>,
}

impl SettingsModalState {
    pub fn new() -> Self {
        Self {
            shell_statuses: Vec::new(),
            checking_shells: false,
            log_file_size: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShellSetupStatus {
    pub shell_type: versi_shell::ShellType,
    pub shell_name: String,
    pub status: ShellVerificationStatus,
    pub configuring: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ShellVerificationStatus {
    Unknown,
    Configured,
    NotConfigured,
    NoConfigFile,
    FunctionalButNotInConfig,
    Error(String),
}
