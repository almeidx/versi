//! Shell detection and shell-config management for Versi.
//!
//! The crate is backend-agnostic: callers provide backend-specific markers and
//! init commands, while this crate handles shell discovery, config edits, and
//! verification flows.
//!
//! High-level capabilities:
//! - Detect native shells (and WSL shells on Windows).
//! - Load/update shell config files with idempotent edits.
//! - Verify whether shell integration is configured and functional.

mod config;
mod detect;
mod verify;

/// Shell config model and edit result used for idempotent file updates.
pub use config::{ConfigError, ShellConfig, ShellConfigEdit};
/// Shell detection models and entry points.
pub use detect::{ShellInfo, ShellType, detect_native_shells, detect_shells, detect_wsl_shells};
/// Verification and configuration helpers used by the app layer.
pub use verify::{
    ShellConfigLoadError, VerificationError, VerificationResult, WslShellConfigError,
    configure_wsl_shell_config, get_or_create_config_path, verify_shell_config,
    verify_wsl_shell_config,
};
/// Shared shell initialization options used across backend integrations.
pub use versi_backend::ShellInitOptions;
