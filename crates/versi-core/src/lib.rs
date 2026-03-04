//! Core cross-crate utilities for Versi.
//!
//! This crate provides reusable logic that is independent of the UI and
//! concrete backend implementations:
//! - Release schedule loading and querying.
//! - Version metadata fetching.
//! - Security advisory fetching and matching helpers.
//! - App update discovery and update payload types.
//! - Small platform command helpers (for example window-hiding adapters).

pub mod auto_update;
pub mod commands;
pub(crate) mod http;
mod install_script;
mod metadata;
mod schedule;
mod security;
mod update;

/// Extension trait that normalizes "hide window" behavior on supported command
/// types.
pub use commands::HideWindow;
/// Installer script download helper with retry/verification policy.
pub use install_script::{
    InstallScriptError, download_install_script_unverified, download_install_script_verified,
    temp_script_path,
};
/// Release metadata model and fetch helper.
pub use metadata::{MetadataError, VersionMeta, fetch_version_metadata};
/// Node release schedule model and fetch helper.
pub use schedule::{ReleaseSchedule, ScheduleError, fetch_release_schedule};
/// Node security advisories model and fetch helper.
pub use security::{SecurityAdvisory, SecurityAdvisoryError, fetch_security_advisories};
/// App update model, GitHub release mapping, and version comparison helpers.
pub use update::{
    AppUpdate, BackendUpdateInfo, GitHubRelease, UpdateError, backend_update_from_release,
    check_for_update, check_github_release, is_newer_version,
};
