mod backend;
mod detection;
mod error;
mod progress;
mod schedule;
mod update;
mod version;

pub mod commands;
pub use commands::HideWindow;

pub use backend::{Environment, FnmBackend};
pub use detection::{detect_fnm, detect_fnm_dir, install_fnm, FnmDetection};
pub use error::FnmError;
pub use progress::parse_progress_line;
pub use schedule::{fetch_release_schedule, ReleaseSchedule};
pub use update::{check_for_update, AppUpdate};
pub use version::{parse_installed_versions, parse_remote_versions};

pub use versi_backend::{
    BackendError, BackendInfo, InstallPhase, InstallProgress, InstalledVersion,
    ManagerCapabilities, NodeVersion, RemoteVersion, ShellInitOptions, VersionGroup,
    VersionManager, VersionParseError,
};
