use std::collections::HashMap;
use std::path::PathBuf;

use versi_backend::{BackendUpdate, InstallProgress, InstalledVersion, RemoteVersion};
use versi_core::{AppUpdate, ReleaseSchedule, VersionMeta};
use versi_platform::EnvironmentId;
use versi_shell::ShellType;

use crate::backend_kind::BackendKind;
use crate::error::AppError;
use crate::settings::{AppUpdateBehavior, TrayBehavior};
use crate::state::SearchFilter;
use crate::tray::TrayMessage;

#[derive(Debug, Clone)]
pub enum Message {
    NoOp,
    Initialized(Box<InitResult>),

    EnvironmentSelected(usize),
    SelectNextEnvironment,
    SelectPreviousEnvironment,
    EnvironmentLoaded {
        env_id: EnvironmentId,
        request_seq: u64,
        result: Result<Vec<InstalledVersion>, AppError>,
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
    SearchFilterToggled(SearchFilter),

    FetchRemoteVersions,
    RemoteVersionsFetched {
        request_seq: u64,
        result: Result<Vec<RemoteVersion>, AppError>,
    },
    ReleaseScheduleFetched {
        request_seq: u64,
        result: Box<Result<ReleaseSchedule, AppError>>,
    },

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
        error: Option<AppError>,
    },

    RequestUninstall(String),
    ConfirmUninstallDefault(String),
    UninstallComplete {
        version: String,
        success: bool,
        error: Option<AppError>,
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
    CancelBulkRun,

    SetDefault(String),
    DefaultChanged {
        success: bool,
        error: Option<AppError>,
    },

    ToastDismiss(usize),

    NavigateToVersions,
    NavigateToSettings,
    NavigateToAbout,
    VersionRowHovered(Option<String>),
    ThemeChanged(crate::settings::ThemeSetting),
    AppUpdateBehaviorChanged(AppUpdateBehavior),
    ShellOptionUseOnCdToggled(bool),
    ShellOptionResolveEnginesToggled(bool),
    ShellOptionCorepackEnabledToggled(bool),
    DebugLoggingToggled(bool),
    CopyToClipboard(String),
    ClearLogFile,
    LogFileCleared,
    RevealLogFile,
    RevealSettingsFile,
    LogFileStatsLoaded(Option<u64>),
    ShellSetupChecked(Vec<(ShellType, versi_shell::VerificationResult)>),
    ConfigureShell(ShellType),
    ShellConfigured(ShellType, Result<(), AppError>),
    ShellFlagsUpdated,

    ExportSettings,
    SettingsExported(Result<std::path::PathBuf, AppError>),
    ImportSettings,
    SettingsImported(Result<(), AppError>),

    PreferredBackendChanged(BackendKind),

    OnboardingNext,
    OnboardingBack,
    OnboardingSelectBackend(BackendKind),
    OnboardingInstallBackend,
    OnboardingBackendInstallResult(Result<(), AppError>),
    OnboardingConfigureShell(ShellType),
    OnboardingShellConfigResult(Result<(), AppError>),
    OnboardingComplete,

    AnimationTick,
    Tick,
    WindowEvent(iced::window::Event),
    CloseWindow,
    HideDockIcon,

    TrayEvent(TrayMessage),
    TrayBehaviorChanged(TrayBehavior),
    StartMinimizedToggled(bool),
    LaunchAtLoginToggled(bool),
    WindowOpened(iced::window::Id),

    AppUpdateChecked(Box<Result<Option<AppUpdate>, AppError>>),
    OpenAppUpdate,
    StartAppUpdate,
    AppUpdateProgress {
        downloaded: u64,
        total: u64,
    },
    AppUpdateExtracting,
    AppUpdateApplying,
    AppUpdateComplete(Box<Result<versi_core::auto_update::ApplyResult, AppError>>),
    RestartApp,
    BackendUpdateChecked(Box<Result<Option<BackendUpdate>, AppError>>),
    OpenBackendUpdate,

    FetchReleaseSchedule,
    FetchVersionMetadata,
    VersionMetadataFetched {
        request_seq: u64,
        result: Box<Result<HashMap<String, VersionMeta>, AppError>>,
    },
    ShowVersionDetail(String),

    VersionListCursorMoved(iced::Point),
    ShowContextMenu {
        version: String,
        is_installed: bool,
        is_default: bool,
    },
    CloseContextMenu,

    ShowKeyboardShortcuts,
    OpenLink(String),

    SystemThemeChanged(iced::theme::Mode),
}

#[derive(Debug, Clone)]
pub struct InitResult {
    pub backend_found: bool,
    pub backend_path: Option<PathBuf>,
    pub backend_dir: Option<PathBuf>,
    pub backend_version: Option<String>,
    pub environments: Vec<EnvironmentInfo>,
    pub detected_backends: Vec<BackendKind>,
}

#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    pub id: EnvironmentId,
    pub backend_name: BackendKind,
    pub backend_version: Option<String>,
    pub available: bool,
    pub unavailable_reason: Option<String>,
}
