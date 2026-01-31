mod backend;
mod detection;
mod error;
mod progress;
mod provider;
mod update;
mod version;

pub use backend::{Environment, FnmBackend};
pub use error::FnmError;
pub use progress::parse_progress_line;
pub use provider::FnmProvider;
pub use version::{parse_installed_versions, parse_remote_versions};
