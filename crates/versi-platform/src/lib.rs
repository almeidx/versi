mod environment;
mod paths;

#[cfg(target_os = "windows")]
mod wsl;

pub use environment::{Environment, EnvironmentId};
pub use paths::AppPaths;

#[cfg(target_os = "windows")]
pub use wsl::{check_fnm_in_wsl, detect_wsl_distros, execute_in_wsl, WslDistro};
