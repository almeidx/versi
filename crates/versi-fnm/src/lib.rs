mod backend;
mod detection;
mod provider;
mod update;
mod version;

pub use backend::FnmBackend;
pub use provider::FnmProvider;
pub use version::{parse_installed_versions, parse_remote_versions};
