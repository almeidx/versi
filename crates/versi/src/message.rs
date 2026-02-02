use std::path::PathBuf;

use versi_backend::{BackendUpdate, InstalledVersion, RemoteVersion};
use versi_core::{AppUpdate, ReleaseSchedule};
use versi_platform::EnvironmentId;
use versi_shell::ShellType;

use crate::settings::TrayBehavior;
use crate::tray::TrayMessage;

#[derive(Debug, Clone)]
pub enum Message {
    NoOp,
    Initialized(InitResult),

    EnvironmentSelected(usize),
    EnvironmentLoaded {
        env_id: EnvironmentId,
        versions: Vec<InstalledVersion>,
    },
    RefreshEnvironment,
    FocusSearch,
    SelectPreviousVersion,
    SelectNextVersion,
    ActivateSelectedVersion,

    VersionGroupToggled {
        major: u32,
    },
    SearchChanged(String),

    FetchRemoteVersions,
    RemoteVersionsFetched(Result<Vec<RemoteVersion>, String>),
    ReleaseScheduleFetched(Result<ReleaseSchedule, String>),

    CloseModal,
    OpenChangelog(String),
    StartInstall(String),
    InstallComplete {
        version: String,
        success: bool,
        error: Option<String>,
    },

    RequestUninstall(String),
    UninstallComplete {
        version: String,
        success: bool,
        error: Option<String>,
    },

    RequestBulkUpdateMajors,
    RequestBulkUninstallEOL,
    RequestBulkUninstallMajor {
        major: u32,
    },
    RequestBulkUninstallMajorExceptLatest {
        major: u32,
    },
    ConfirmBulkUpdateMajors,
    ConfirmBulkUninstallEOL,
    ConfirmBulkUninstallMajor {
        major: u32,
    },
    ConfirmBulkUninstallMajorExceptLatest {
        major: u32,
    },
    CancelBulkOperation,

    SetDefault(String),
    DefaultChanged {
        success: bool,
        error: Option<String>,
    },

    ToastDismiss(usize),

    NavigateToVersions,
    NavigateToSettings,
    NavigateToAbout,
    VersionRowHovered(Option<String>),
    ThemeChanged(crate::settings::ThemeSetting),
    ShellOptionUseOnCdToggled(bool),
    ShellOptionResolveEnginesToggled(bool),
    ShellOptionCorepackEnabledToggled(bool),
    DebugLoggingToggled(bool),
    CopyToClipboard(String),
    ClearLogFile,
    LogFileCleared,
    RevealLogFile,
    LogFileStatsLoaded(Option<u64>),
    ShellSetupChecked(Vec<(ShellType, versi_shell::VerificationResult)>),
    ConfigureShell(ShellType),
    ShellConfigured(ShellType, Result<(), String>),
    ShellFlagsUpdated,

    PreferredBackendChanged(String),

    OnboardingNext,
    OnboardingBack,
    OnboardingSelectBackend(String),
    OnboardingInstallBackend,
    OnboardingBackendInstallResult(Result<(), String>),
    OnboardingConfigureShell(ShellType),
    OnboardingShellConfigResult(Result<(), String>),
    OnboardingComplete,

    AnimationTick,
    Tick,
    WindowEvent(iced::window::Event),
    CloseWindow,
    HideDockIcon,

    TrayEvent(TrayMessage),
    TrayBehaviorChanged(TrayBehavior),
    StartMinimizedToggled(bool),
    WindowOpened(iced::window::Id),

    AppUpdateChecked(Result<Option<AppUpdate>, String>),
    OpenAppUpdate,
    BackendUpdateChecked(Result<Option<BackendUpdate>, String>),
    OpenBackendUpdate,

    FetchReleaseSchedule,

    OpenLink(String),
}

#[derive(Debug, Clone)]
pub struct InitResult {
    pub backend_found: bool,
    pub backend_path: Option<PathBuf>,
    pub backend_dir: Option<PathBuf>,
    pub backend_version: Option<String>,
    pub environments: Vec<EnvironmentInfo>,
    pub detected_backends: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    pub id: EnvironmentId,
    pub backend_name: &'static str,
    pub backend_version: Option<String>,
    pub available: bool,
    pub unavailable_reason: Option<String>,
}
