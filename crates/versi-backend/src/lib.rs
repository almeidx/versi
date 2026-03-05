//! Backend abstraction layer for Versi.
//!
//! This crate defines the interfaces and shared data models that concrete
//! backend implementations (for example fnm or nvm) must implement.
//!
//! Public API groups:
//! - `BackendProvider`: detection, install, update checks, and manager creation.
//! - `VersionManager`: operational backend API (list/install/uninstall/default).
//! - Shared types: version models and grouping helpers used by the GUI layer.

mod command;
mod error;
mod helpers;
mod test_macros;
mod text;
mod traits;
mod types;

/// Shared command building and execution for native/WSL dispatch.
pub use command::{
    CommandEnvironment, build_backend_command, combine_error_output, command_output_to_result,
    execute_backend_command, execute_backend_command_with,
};
/// Error type shared by backend providers and managers.
pub use error::BackendError;
#[cfg(unix)]
pub use helpers::run_unix_install_script;
/// Shared helpers for common backend patterns.
pub use helpers::{
    download_and_prepare_install_script, find_default_version, parse_current_version,
    strip_version_prefix,
};
/// Terminal text sanitization utilities.
pub use text::sanitize_terminal_text;
/// Backend traits and capability metadata used by the application.
pub use traits::{
    BackendDetection, BackendInfo, BackendProvider, BackendUpdate, InstallProgress,
    ManagerCapabilities, ShellInitOptions, VersionManager,
};
/// Version and grouping models shared across backend implementations.
pub use types::{InstalledVersion, NodeVersion, RemoteVersion, VersionGroup};
