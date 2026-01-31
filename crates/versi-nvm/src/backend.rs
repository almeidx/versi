use async_trait::async_trait;
use log::{debug, info};
use std::path::PathBuf;
use tokio::sync::mpsc;

use versi_backend::{
    BackendError, BackendInfo, InstallProgress, InstalledVersion, ManagerCapabilities, NodeVersion,
    RemoteVersion, ShellInitOptions, VersionManager,
};

use crate::client::{NvmClient, NvmEnvironment};

#[derive(Clone)]
pub struct NvmBackend {
    info: BackendInfo,
    client: NvmClient,
}

impl NvmBackend {
    pub fn new(client: NvmClient, version: Option<String>) -> Self {
        let (path, data_dir) = match &client.environment {
            NvmEnvironment::Unix { nvm_dir } => (nvm_dir.join("nvm.sh"), Some(nvm_dir.clone())),
            NvmEnvironment::Windows { nvm_exe } => {
                (nvm_exe.clone(), nvm_exe.parent().map(|p| p.to_path_buf()))
            }
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
}

impl std::fmt::Debug for NvmBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NvmBackend")
            .field("info", &self.info)
            .finish()
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
            supports_progress: false,
            supports_lts_filter: true,
            supports_use_version: true,
            supports_shell_integration: supports_shell,
            supports_auto_switch: false,
            supports_corepack: false,
            supports_resolve_engines: false,
        }
    }

    fn backend_info(&self) -> &BackendInfo {
        &self.info
    }

    async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError> {
        debug!("nvm: listing installed versions");
        self.client
            .list_installed()
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        debug!("nvm: listing remote versions");
        self.client
            .list_remote()
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    async fn list_remote_lts(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        debug!("nvm: listing remote LTS versions");
        self.client
            .list_remote_lts()
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        debug!("nvm: getting current version");
        self.client
            .current()
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        debug!("nvm: getting default version");
        self.client
            .default_version()
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    async fn install(&self, version: &str) -> Result<(), BackendError> {
        info!("nvm: installing version {}", version);
        self.client
            .install(version)
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    async fn install_with_progress(
        &self,
        version: &str,
    ) -> Result<mpsc::UnboundedReceiver<InstallProgress>, BackendError> {
        info!("nvm: installing version {} with progress", version);
        self.client
            .install_with_progress(version)
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    async fn uninstall(&self, version: &str) -> Result<(), BackendError> {
        info!("nvm: uninstalling version {}", version);
        self.client
            .uninstall(version)
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    async fn set_default(&self, version: &str) -> Result<(), BackendError> {
        info!("nvm: setting default version to {}", version);
        self.client
            .set_default(version)
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    async fn use_version(&self, version: &str) -> Result<(), BackendError> {
        info!("nvm: using version {}", version);
        self.client
            .use_version(version)
            .await
            .map_err(|e| BackendError::CommandFailed {
                stderr: e.to_string(),
            })
    }

    fn shell_init_command(&self, _shell: &str, _options: &ShellInitOptions) -> Option<String> {
        match &self.client.environment {
            NvmEnvironment::Unix { nvm_dir } => Some(format!(
                "export NVM_DIR=\"{}\" && [ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\"",
                nvm_dir.display()
            )),
            NvmEnvironment::Wsl { nvm_dir, .. } => Some(format!(
                "export NVM_DIR=\"{}\" && [ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\"",
                nvm_dir
            )),
            NvmEnvironment::Windows { .. } => None,
        }
    }
}
