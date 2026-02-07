mod commands;
mod environment;
mod paths;

#[cfg(target_os = "windows")]
mod wsl;

pub use commands::HideWindow;
pub use environment::{Environment, EnvironmentId};
pub use paths::AppPaths;

#[cfg(target_os = "windows")]
pub use wsl::{WslDistro, detect_wsl_distros, execute_in_wsl};
