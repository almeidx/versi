use async_trait::async_trait;
use std::path::PathBuf;

use crate::error::BackendError;
use crate::types::{InstalledVersion, NodeVersion, RemoteVersion};

#[derive(Debug, Clone)]
pub struct BackendDetection {
    pub found: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub in_path: bool,
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct BackendUpdate {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
}

#[async_trait]
pub trait BackendProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn shell_config_marker(&self) -> &str;
    fn shell_config_label(&self) -> &str;
    async fn detect(&self) -> BackendDetection;
    async fn install_backend(&self) -> Result<(), BackendError>;
    async fn check_for_update(
        &self,
        client: &reqwest::Client,
        current_version: &str,
    ) -> Result<Option<BackendUpdate>, String>;
    fn create_manager(&self, detection: &BackendDetection) -> Box<dyn VersionManager>;
    fn create_manager_for_wsl(
        &self,
        distro: String,
        backend_path: String,
    ) -> Box<dyn VersionManager>;

    fn wsl_search_paths(&self) -> Vec<&'static str> {
        vec![]
    }
}

#[derive(Debug, Clone, Default)]
pub struct ManagerCapabilities {
    pub supports_lts_filter: bool,
    pub supports_use_version: bool,
    pub supports_shell_integration: bool,
    pub supports_auto_switch: bool,
    pub supports_corepack: bool,
    pub supports_resolve_engines: bool,
}

#[derive(Debug, Clone)]
pub struct BackendInfo {
    pub name: &'static str,
    pub path: PathBuf,
    pub version: Option<String>,
    pub data_dir: Option<PathBuf>,
    pub in_path: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ShellInitOptions {
    pub use_on_cd: bool,
    pub resolve_engines: bool,
    pub corepack_enabled: bool,
}

#[async_trait]
pub trait VersionManager: Send + Sync + VersionManagerClone {
    fn name(&self) -> &'static str;

    fn capabilities(&self) -> ManagerCapabilities;

    fn backend_info(&self) -> &BackendInfo;

    async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError>;

    async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError>;

    async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError>;

    async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError>;

    async fn install(&self, version: &str) -> Result<(), BackendError>;

    async fn uninstall(&self, version: &str) -> Result<(), BackendError>;

    async fn set_default(&self, version: &str) -> Result<(), BackendError>;

    async fn use_version(&self, _version: &str) -> Result<(), BackendError> {
        Err(BackendError::Unsupported("use_version".to_string()))
    }

    async fn list_remote_lts(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        let all = self.list_remote().await?;
        Ok(all
            .into_iter()
            .filter(|v| v.lts_codename.is_some())
            .collect())
    }

    fn shell_init_command(&self, shell: &str, options: &ShellInitOptions) -> Option<String>;
}

pub trait VersionManagerClone: Send + Sync {
    fn clone_box(&self) -> Box<dyn VersionManager>;
}

impl<T> VersionManagerClone for T
where
    T: 'static + VersionManager + Clone,
{
    fn clone_box(&self) -> Box<dyn VersionManager> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn VersionManager> {
    fn clone(&self) -> Box<dyn VersionManager> {
        self.clone_box()
    }
}

impl<T: VersionManager + Clone + 'static> From<T> for Box<dyn VersionManager> {
    fn from(manager: T) -> Self {
        Box::new(manager)
    }
}
