pub mod commands;
mod schedule;
mod update;

pub use commands::HideWindow;
pub use schedule::{ReleaseSchedule, fetch_release_schedule};
pub use update::{AppUpdate, GitHubRelease, check_for_update, is_newer_version};
