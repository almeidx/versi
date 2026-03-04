mod commands;
mod environment;
mod paths;

#[cfg(target_os = "windows")]
mod wsl;

pub use commands::HideWindow;
pub use environment::{Environment, EnvironmentId};
pub use paths::{AppPaths, AppPathsError};

#[cfg(target_os = "windows")]
pub use wsl::{WslDistro, WslTimeouts, detect_wsl_distros, execute_in_wsl};

pub const APP_ID: &str = "dev.almeidx.versi";
pub const DESKTOP_ENTRY_FILENAME: &str = "dev.almeidx.versi.desktop";
pub const LAUNCHAGENT_PLIST_FILENAME: &str = "dev.almeidx.versi.plist";
