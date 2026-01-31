use async_trait::async_trait;

use versi_backend::{
    BackendDetection, BackendError, BackendProvider, BackendUpdate, VersionManager,
};

use crate::backend::FnmBackend;
use crate::detection::{detect_fnm, detect_fnm_dir, install_fnm};
use crate::update::check_for_fnm_update;

#[derive(Default)]
pub struct FnmProvider;

impl FnmProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BackendProvider for FnmProvider {
    fn name(&self) -> &'static str {
        "fnm"
    }

    fn display_name(&self) -> &'static str {
        "fnm (Fast Node Manager)"
    }

    fn shell_config_marker(&self) -> &str {
        "fnm env"
    }

    fn shell_config_label(&self) -> &str {
        "fnm (Fast Node Manager)"
    }

    async fn detect(&self) -> BackendDetection {
        let detection = detect_fnm().await;
        BackendDetection {
            found: detection.found,
            path: detection.path,
            version: detection.version,
            in_path: detection.in_path,
            data_dir: detection.fnm_dir,
        }
    }

    async fn install_backend(&self) -> Result<(), BackendError> {
        install_fnm()
            .await
            .map_err(|e| BackendError::InstallFailed(e.to_string()))
    }

    async fn check_for_update(
        &self,
        client: &reqwest::Client,
        current_version: &str,
    ) -> Result<Option<BackendUpdate>, String> {
        check_for_fnm_update(client, current_version).await
    }

    fn create_manager(&self, detection: &BackendDetection) -> Box<dyn VersionManager> {
        let path = detection
            .path
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("fnm"));
        let data_dir = detection.data_dir.clone().or_else(detect_fnm_dir);
        let backend = FnmBackend::new(path, detection.version.clone(), data_dir.clone());
        let backend = if let Some(dir) = data_dir {
            backend.with_fnm_dir(dir)
        } else {
            backend
        };
        Box::new(backend)
    }

    fn create_manager_for_wsl(
        &self,
        distro: String,
        backend_path: String,
    ) -> Box<dyn VersionManager> {
        Box::new(FnmBackend::with_wsl(distro, backend_path))
    }

    fn wsl_search_paths(&self) -> Vec<&'static str> {
        vec![
            "$HOME/.local/share/fnm/fnm",
            "$HOME/.cargo/bin/fnm",
            "/usr/local/bin/fnm",
            "/usr/bin/fnm",
            "$HOME/.fnm/fnm",
        ]
    }
}
