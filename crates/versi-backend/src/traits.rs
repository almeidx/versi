use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

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

impl From<versi_core::BackendUpdateInfo> for BackendUpdate {
    fn from(info: versi_core::BackendUpdateInfo) -> Self {
        Self {
            current_version: info.current_version,
            latest_version: info.latest_version,
            release_url: info.release_url,
        }
    }
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
        detection: &BackendDetection,
    ) -> Result<Option<BackendUpdate>, BackendError>;
    fn create_manager(&self, detection: &BackendDetection) -> Arc<dyn VersionManager>;
    fn create_manager_for_wsl(
        &self,
        distro: String,
        backend_path: String,
    ) -> Arc<dyn VersionManager>;

    fn wsl_search_paths(&self) -> &'static [&'static str] {
        &[]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct ManagerCapabilities {
    pub supports_lts_filter: bool,
    pub supports_use_version: bool,
    pub supports_shell_integration: bool,
    pub supports_auto_switch: bool,
    pub supports_corepack: bool,
    pub supports_resolve_engines: bool,
    pub supports_uninstall: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallProgress {
    Downloading {
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    Extracting,
    Configuring,
}

#[async_trait]
pub trait VersionManager: Send + Sync {
    fn name(&self) -> &'static str;

    fn capabilities(&self) -> ManagerCapabilities;

    fn backend_info(&self) -> &BackendInfo;

    async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError>;

    async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError>;

    async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError>;

    async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError>;

    async fn install(&self, version: &str) -> Result<(), BackendError>;

    async fn install_with_progress(
        &self,
        version: &str,
        _progress_tx: mpsc::Sender<InstallProgress>,
    ) -> Result<(), BackendError> {
        self.install(version).await
    }

    async fn uninstall(&self, version: &str) -> Result<(), BackendError>;

    async fn set_default(&self, version: &str) -> Result<(), BackendError>;

    async fn use_version(&self, _version: &str) -> Result<(), BackendError> {
        Err(BackendError::Unsupported {
            operation: "use_version",
        })
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use async_trait::async_trait;

    use super::*;

    struct MockManager {
        info: BackendInfo,
        remote: Vec<RemoteVersion>,
    }

    impl MockManager {
        fn new(remote: Vec<RemoteVersion>) -> Self {
            Self {
                info: BackendInfo {
                    name: "mock",
                    path: PathBuf::from("/tmp/mock-backend"),
                    version: Some("1.0.0".to_string()),
                    data_dir: Some(PathBuf::from("/tmp/mock-data")),
                    in_path: true,
                },
                remote,
            }
        }
    }

    #[async_trait]
    impl VersionManager for MockManager {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn capabilities(&self) -> ManagerCapabilities {
            ManagerCapabilities::default()
        }

        fn backend_info(&self) -> &BackendInfo {
            &self.info
        }

        async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError> {
            Ok(Vec::new())
        }

        async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError> {
            Ok(self.remote.clone())
        }

        async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError> {
            Ok(None)
        }

        async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError> {
            Ok(None)
        }

        async fn install(&self, _version: &str) -> Result<(), BackendError> {
            Ok(())
        }

        async fn uninstall(&self, _version: &str) -> Result<(), BackendError> {
            Ok(())
        }

        async fn set_default(&self, _version: &str) -> Result<(), BackendError> {
            Ok(())
        }

        fn shell_init_command(&self, _shell: &str, _options: &ShellInitOptions) -> Option<String> {
            None
        }
    }

    fn remote(version: &str, lts_codename: Option<&str>) -> RemoteVersion {
        RemoteVersion {
            version: version.parse().expect("valid semver in test"),
            lts_codename: lts_codename.map(str::to_string),
            is_latest: false,
        }
    }

    #[tokio::test]
    async fn use_version_default_returns_unsupported() {
        let manager = MockManager::new(Vec::new());

        let result = manager.use_version("v20.0.0").await;

        assert!(
            matches!(
                result,
                Err(BackendError::Unsupported {
                    operation: "use_version"
                })
            ),
            "expected Unsupported(\"use_version\"), got {result:?}"
        );
    }

    #[tokio::test]
    async fn list_remote_lts_filters_non_lts_versions() {
        let manager = MockManager::new(vec![
            remote("v24.0.0", None),
            remote("v22.1.0", Some("Jod")),
            remote("v20.10.0", Some("Iron")),
        ]);

        let lts = manager
            .list_remote_lts()
            .await
            .expect("lts listing succeeds");

        assert_eq!(lts.len(), 2);
        assert_eq!(lts[0].version.to_string(), "v22.1.0");
        assert_eq!(lts[1].version.to_string(), "v20.10.0");
        assert!(lts.iter().all(|v| v.lts_codename.is_some()));
    }

    #[tokio::test]
    async fn arc_clone_preserves_manager_behavior_and_info() {
        let arc: Arc<dyn VersionManager> =
            Arc::new(MockManager::new(vec![remote("v20.1.0", None)]));
        let cloned = arc.clone();

        assert_eq!(cloned.name(), "mock");
        assert_eq!(
            cloned.backend_info().path,
            PathBuf::from("/tmp/mock-backend")
        );
        let remote_versions = cloned
            .list_remote()
            .await
            .expect("list_remote should work on cloned manager");
        assert_eq!(remote_versions.len(), 1);
        assert_eq!(remote_versions[0].version.to_string(), "v20.1.0");
    }
}
