use async_trait::async_trait;
use log::{debug, info};
use std::path::PathBuf;

use versi_backend::{
    BackendError, BackendInfo, InstalledVersion, ManagerCapabilities, NodeVersion, RemoteVersion,
    ShellInitOptions, VersionManager,
};

use crate::client::{NvmClient, NvmEnvironment};

#[derive(Debug, Clone)]
pub struct NvmBackend {
    info: BackendInfo,
    client: NvmClient,
}

impl NvmBackend {
    #[must_use]
    pub fn new(client: NvmClient, version: Option<String>) -> Self {
        let (path, data_dir) = match &client.environment {
            NvmEnvironment::Unix { nvm_dir } => (nvm_dir.join("nvm.sh"), Some(nvm_dir.clone())),
            NvmEnvironment::Windows { nvm_exe } => (
                nvm_exe.clone(),
                nvm_exe.parent().map(std::path::Path::to_path_buf),
            ),
            NvmEnvironment::Wsl { nvm_dir, .. } => (
                PathBuf::from(nvm_dir).join("nvm.sh"),
                Some(PathBuf::from(nvm_dir)),
            ),
        };

        Self {
            info: BackendInfo {
                name: "nvm",
                path,
                version,
                data_dir,
                in_path: true,
            },
            client,
        }
    }

    #[must_use]
    pub fn with_in_path(mut self, in_path: bool) -> Self {
        self.info.in_path = in_path;
        self
    }
}

#[async_trait]
impl VersionManager for NvmBackend {
    fn name(&self) -> &'static str {
        "nvm"
    }

    fn capabilities(&self) -> ManagerCapabilities {
        let supports_shell = !self.client.is_windows();
        ManagerCapabilities {
            supports_lts_filter: true,
            supports_use_version: true,
            supports_shell_integration: supports_shell,
            supports_auto_switch: false,
            supports_corepack: false,
            supports_resolve_engines: false,
            supports_uninstall: true,
        }
    }

    fn backend_info(&self) -> &BackendInfo {
        &self.info
    }

    async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError> {
        debug!("nvm: listing installed versions");
        self.client.list_installed().await
    }

    async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        debug!("nvm: listing remote versions");
        self.client.list_remote().await
    }

    async fn list_remote_lts(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        debug!("nvm: listing remote LTS versions");
        self.client.list_remote_lts().await
    }

    async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        debug!("nvm: getting current version");
        self.client.current().await
    }

    async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        debug!("nvm: getting default version");
        self.client.default_version().await
    }

    async fn install(&self, version: &str) -> Result<(), BackendError> {
        info!("nvm: installing version {version}");
        self.client.install(version).await
    }

    async fn uninstall(&self, version: &str) -> Result<(), BackendError> {
        info!("nvm: uninstalling version {version}");
        self.client.uninstall(version).await
    }

    async fn set_default(&self, version: &str) -> Result<(), BackendError> {
        info!("nvm: setting default version to {version}");
        self.client.set_default(version).await
    }

    async fn use_version(&self, version: &str) -> Result<(), BackendError> {
        info!("nvm: using version {version}");
        self.client.use_version(version).await
    }

    fn shell_init_command(&self, _shell: &str, _options: &ShellInitOptions) -> Option<String> {
        match &self.client.environment {
            NvmEnvironment::Unix { nvm_dir } => Some(format!(
                "export NVM_DIR=\"{}\" && [ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\"",
                nvm_dir.display()
            )),
            NvmEnvironment::Wsl { nvm_dir, .. } => Some(format!(
                "export NVM_DIR=\"{nvm_dir}\" && [ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\""
            )),
            NvmEnvironment::Windows { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unix_backend() -> NvmBackend {
        let client = NvmClient::unix(PathBuf::from("/home/user/.nvm"));
        NvmBackend::new(client, Some("0.40.1".to_string()))
    }

    fn windows_backend() -> NvmBackend {
        let client = NvmClient::windows(PathBuf::from("C:\\nvm\\nvm.exe"));
        NvmBackend::new(client, Some("1.1.12".to_string()))
    }

    #[test]
    fn unix_capabilities_supports_shell_integration() {
        let caps = unix_backend().capabilities();
        assert!(caps.supports_shell_integration);
        assert!(caps.supports_lts_filter);
        assert!(caps.supports_use_version);
        assert!(!caps.supports_auto_switch);
        assert!(!caps.supports_corepack);
        assert!(!caps.supports_resolve_engines);
        assert!(caps.supports_uninstall);
    }

    #[test]
    fn windows_capabilities_no_shell_integration() {
        let caps = windows_backend().capabilities();
        assert!(!caps.supports_shell_integration);
        assert!(caps.supports_lts_filter);
        assert!(caps.supports_use_version);
        assert!(caps.supports_uninstall);
    }
}
