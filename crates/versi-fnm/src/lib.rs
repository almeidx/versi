mod backend;
mod detection;
mod error;
mod provider;
mod update;
mod version;

pub use backend::{Environment, FnmBackend};
pub use error::FnmError;
pub use provider::FnmProvider;
pub use version::{parse_installed_versions, parse_remote_versions};
