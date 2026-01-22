use std::path::PathBuf;
use std::time::Instant;
use versi_core::{
    AppUpdate, BackendInfo, InstallProgress, InstalledVersion, NodeVersion, ReleaseSchedule,
    RemoteVersion, VersionGroup, VersionManager,
};
use versi_platform::EnvironmentId;
use versi_shell::ShellType;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum AppState {
    Loading,
    Onboarding(OnboardingState),
    Main(MainState),
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct OnboardingState {
    pub step: OnboardingStep,
    pub fnm_installing: bool,
    pub install_error: Option<String>,
    pub detected_shells: Vec<ShellConfigStatus>,
    pub selected_version: Option<String>,
    pub available_lts_versions: Vec<RemoteVersion>,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Welcome,
            fnm_installing: false,
            install_error: None,
            detected_shells: Vec::new(),
            selected_version: None,
            available_lts_versions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    Welcome,
    InstallFnm,
    ConfigureShell,
    InstallNode,
    Complete,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    pub current_operation: Option<Operation>,
    pub toasts: Vec<Toast>,
    pub modal: Option<Modal>,
    pub search_query: String,
    pub backend: Box<dyn VersionManager>,
    pub app_update: Option<AppUpdate>,
}

impl std::fmt::Debug for MainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainState")
            .field("environments", &self.environments)
            .field("active_environment_idx", &self.active_environment_idx)
            .field("available_versions", &self.available_versions)
            .field("current_operation", &self.current_operation)
            .field("toasts", &self.toasts)
            .field("modal", &self.modal)
            .field("search_query", &self.search_query)
            .field("backend", &self.backend.name())
            .field("app_update", &self.app_update)
            .finish()
    }
}

impl MainState {
    pub fn new(backend: Box<dyn VersionManager>) -> Self {
        Self {
            environments: vec![EnvironmentState::new(EnvironmentId::Native)],
            active_environment_idx: 0,
            available_versions: VersionCache::new(),
            current_operation: None,
            toasts: Vec::new(),
            modal: None,
            search_query: String::new(),
            backend,
            app_update: None,
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
            current_operation: None,
            toasts: Vec::new(),
            modal: None,
            search_query: String::new(),
            backend,
            app_update: None,
        }
    }

    #[allow(dead_code)]
    pub fn backend(&self) -> &dyn VersionManager {
        self.backend.as_ref()
    }

    #[allow(dead_code)]
    pub fn backend_info(&self) -> &BackendInfo {
        self.backend.backend_info()
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
#[allow(dead_code)]
pub struct EnvironmentState {
    pub id: EnvironmentId,
    pub name: String,
    pub installed_versions: Vec<InstalledVersion>,
    pub version_groups: Vec<VersionGroup>,
    pub default_version: Option<NodeVersion>,
    pub loading: bool,
    pub error: Option<String>,
}

impl EnvironmentState {
    pub fn new(id: EnvironmentId) -> Self {
        let name = id.display_name();
        Self {
            id,
            name,
            installed_versions: Vec::new(),
            version_groups: Vec::new(),
            default_version: None,
            loading: true,
            error: None,
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
}

#[allow(dead_code)]
impl VersionCache {
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
            fetched_at: None,
            loading: false,
            error: None,
            schedule: None,
        }
    }

    pub fn is_active(&self, major: u32) -> bool {
        self.schedule
            .as_ref()
            .map(|s| s.is_active(major))
            .unwrap_or(true)
    }

    pub fn is_lts(&self, major: u32) -> bool {
        self.schedule
            .as_ref()
            .map(|s| s.is_lts(major))
            .unwrap_or(major.is_multiple_of(2))
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
        previous: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: usize,
    pub message: String,
    pub status: ToastStatus,
    pub undo_action: Option<UndoAction>,
    pub created_at: Instant,
}

impl Toast {
    pub fn success(id: usize, message: String) -> Self {
        Self {
            id,
            message,
            status: ToastStatus::Success,
            undo_action: None,
            created_at: Instant::now(),
        }
    }

    pub fn error(id: usize, message: String) -> Self {
        Self {
            id,
            message,
            status: ToastStatus::Error,
            undo_action: None,
            created_at: Instant::now(),
        }
    }

    pub fn with_undo(mut self, action: UndoAction) -> Self {
        self.undo_action = Some(action);
        self
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > 5
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ToastStatus {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub enum UndoAction {
    Reinstall { version: String },
    ResetDefault { version: String },
}

#[derive(Debug, Clone)]
pub enum Modal {
    Install(InstallModalState),
    Settings(SettingsModalState),
    ConfirmUninstall { version: String },
}

#[derive(Debug, Clone)]
pub struct SettingsModalState {
    pub shell_statuses: Vec<ShellSetupStatus>,
    pub checking_shells: bool,
}

impl SettingsModalState {
    pub fn new() -> Self {
        Self {
            shell_statuses: Vec::new(),
            checking_shells: false,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ShellSetupStatus {
    pub shell_type: versi_shell::ShellType,
    pub shell_name: String,
    pub status: ShellVerificationStatus,
    pub configuring: bool,
    pub detected_options: Option<versi_shell::FnmShellOptions>,
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InstallModalState {
    pub search_query: String,
    pub filtered_versions: Vec<RemoteVersion>,
    pub selected_version: Option<String>,
    pub loading: bool,
    pub schedule: Option<ReleaseSchedule>,
}

impl InstallModalState {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            filtered_versions: Vec::new(),
            selected_version: None,
            loading: false,
            schedule: None,
        }
    }
}
