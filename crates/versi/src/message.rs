use std::path::PathBuf;

use versi_core::{
    AppUpdate, FnmUpdate, InstallProgress, InstalledVersion, NodeVersion, ReleaseSchedule,
    RemoteVersion,
};
use versi_platform::EnvironmentId;
use versi_shell::ShellType;

use crate::settings::TrayBehavior;
use crate::tray::TrayMessage;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    NoOp,
    Initialized(InitResult),

    EnvironmentSelected(usize),
    EnvironmentLoaded {
        env_id: EnvironmentId,
        versions: Vec<InstalledVersion>,
        default_version: Option<NodeVersion>,
    },
    EnvironmentError {
        env_id: EnvironmentId,
        error: String,
    },
    RefreshEnvironment,

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
    InstallProgress {
        version: String,
        progress: InstallProgress,
    },
    InstallComplete {
        version: String,
        success: bool,
        error: Option<String>,
    },

    RequestUninstall(String),
    ConfirmUninstall(String),
    CancelUninstall,
    CancelQueuedOperation(usize),
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
        version: String,
        previous: Option<String>,
        success: bool,
        error: Option<String>,
    },

    ToastDismiss(usize),
    ToastUndo(usize),

    NavigateToVersions,
    NavigateToSettings,
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
    CheckShellSetup,
    ShellSetupChecked(Vec<(ShellType, versi_shell::VerificationResult)>),
    ConfigureShell(ShellType),
    ShellConfigured(ShellType, Result<(), String>),
    ShellFlagsUpdated(Result<usize, String>),

    OnboardingNext,
    OnboardingBack,
    OnboardingInstallFnm,
    OnboardingFnmInstallResult(Result<(), String>),
    OnboardingConfigureShell(ShellType),
    OnboardingShellConfigResult(Result<(), String>),
    OnboardingComplete,

    Tick,
    WindowEvent(iced::window::Event),
    CloseWindow,
    HideDockIcon,

    TrayEvent(TrayMessage),
    TrayBehaviorChanged(TrayBehavior),
    StartMinimizedToggled(bool),
    WindowOpened(iced::window::Id),

    CheckForAppUpdate,
    AppUpdateChecked(Option<AppUpdate>),
    OpenAppUpdate,
    DismissAppUpdate,

    CheckForFnmUpdate,
    FnmUpdateChecked(Option<FnmUpdate>),
    OpenFnmUpdate,

    OpenLink(String),
}

#[derive(Debug, Clone)]
pub struct InitResult {
    pub fnm_found: bool,
    pub fnm_path: Option<PathBuf>,
    pub fnm_dir: Option<PathBuf>,
    pub fnm_version: Option<String>,
    pub environments: Vec<EnvironmentInfo>,
}

#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    pub id: EnvironmentId,
    pub fnm_version: Option<String>,
    pub available: bool,
    pub unavailable_reason: Option<String>,
}
