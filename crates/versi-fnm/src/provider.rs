use std::sync::Arc;

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
    #[must_use]
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

    fn shell_config_marker(&self) -> &'static str {
        "fnm env"
    }

    fn shell_config_label(&self) -> &'static str {
        "fnm (Fast Node Manager)"
    }

    async fn detect(&self) -> BackendDetection {
        detect_fnm().await
    }

    async fn install_backend(&self) -> Result<(), BackendError> {
        install_fnm().await
    }

    async fn check_for_update(
        &self,
        client: &reqwest::Client,
        current_version: &str,
        _detection: &BackendDetection,
    ) -> Result<Option<BackendUpdate>, BackendError> {
        check_for_fnm_update(client, current_version).await
    }

    fn create_manager(&self, detection: &BackendDetection) -> Arc<dyn VersionManager> {
        let path = detection
            .path
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("fnm"));
        let data_dir = detection.data_dir.clone().or_else(detect_fnm_dir);
        let backend = FnmBackend::new(path, detection.version.clone(), data_dir)
            .with_in_path(detection.in_path);
        Arc::new(backend)
    }

    fn create_manager_for_wsl(
        &self,
        distro: String,
        backend_path: String,
    ) -> Arc<dyn VersionManager> {
        Arc::new(FnmBackend::with_wsl(distro, backend_path).with_in_path(false))
    }

    fn wsl_search_paths(&self) -> &'static [&'static str] {
        &[
            "$HOME/.local/share/fnm/fnm",
            "$HOME/.cargo/bin/fnm",
            "/usr/local/bin/fnm",
            "/usr/bin/fnm",
            "$HOME/.fnm/fnm",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::FnmProvider;

    versi_backend::provider_tests! {
        provider: FnmProvider::new(),
        binary_name: "fnm",
        metadata: {
            name: "fnm",
            display_name: "fnm (Fast Node Manager)",
            shell_config_marker: "fnm env",
            shell_config_label: "fnm (Fast Node Manager)",
        },
        create_manager: {
            path: "/opt/homebrew/bin/fnm",
            version: "1.38.0",
            data_dir: "/tmp/fnm-data",
        },
        wsl_binary_path: "/usr/bin/fnm",
    }
}
