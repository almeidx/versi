mod backend;
mod client;
mod detection;
mod error;
mod provider;
mod update;
mod version;

pub use backend::NvmBackend;
pub use client::{NvmClient, NvmEnvironment};
pub use detection::{NvmDetection, NvmVariant};
pub use error::NvmError;
pub use provider::NvmProvider;

pub use versi_backend::{
    BackendDetection, BackendError, BackendInfo, BackendProvider, BackendUpdate, InstalledVersion,
    ManagerCapabilities, NodeVersion, RemoteVersion, ShellInitOptions, VersionManager,
};
