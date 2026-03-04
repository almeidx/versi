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
mod text;
mod traits;
mod types;

/// Shared command output processing.
pub use command::command_output_to_result;
/// Error type shared by backend providers and managers.
pub use error::{BackendError, NetworkStage};
/// Shared helpers for common backend patterns.
pub use helpers::parse_current_version;
/// Terminal text sanitization utilities.
pub use text::sanitize_terminal_text;
/// Backend traits and capability metadata used by the application.
pub use traits::{
    BackendDetection, BackendInfo, BackendProvider, BackendUpdate, InstallProgress,
    ManagerCapabilities, ShellInitOptions, VersionManager,
};
/// Version and grouping models shared across backend implementations.
pub use types::{
    InstalledVersion, NodeVersion, RemoteVersion, VersionComponent, VersionGroup, VersionParseError,
};
