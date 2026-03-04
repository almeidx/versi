use std::sync::Arc;

use async_trait::async_trait;

use versi_backend::{
    BackendDetection, BackendError, BackendProvider, BackendUpdate, VersionManager,
};

use crate::backend::AsdfBackend;
use crate::detection::{detect_asdf, detect_asdf_data_dir, install_asdf};
use crate::update::check_for_asdf_update;

pub struct AsdfProvider {
    http_client: reqwest::Client,
}

impl AsdfProvider {
    #[must_use]
    pub fn new(http_client: reqwest::Client) -> Self {
        Self { http_client }
    }
}

#[async_trait]
impl BackendProvider for AsdfProvider {
    fn name(&self) -> &'static str {
        "asdf"
    }

    fn display_name(&self) -> &'static str {
        "asdf (asdf-nodejs)"
    }

    fn shell_config_marker(&self) -> &'static str {
        "ASDF_DATA_DIR"
    }

    fn shell_config_label(&self) -> &'static str {
        "asdf (asdf-nodejs)"
    }

    async fn detect(&self) -> BackendDetection {
        detect_asdf().await
    }

    async fn install_backend(&self) -> Result<(), BackendError> {
        install_asdf(&self.http_client).await
    }

    async fn check_for_update(
        &self,
        client: &reqwest::Client,
        current_version: &str,
        _detection: &BackendDetection,
    ) -> Result<Option<BackendUpdate>, BackendError> {
        check_for_asdf_update(client, current_version).await
    }

    fn create_manager(&self, detection: &BackendDetection) -> Arc<dyn VersionManager> {
        let path = detection
            .path
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("asdf"));
        let data_dir = detection.data_dir.clone().or_else(detect_asdf_data_dir);
        let backend = AsdfBackend::new(path, detection.version.clone(), data_dir.clone())
            .with_in_path(detection.in_path);
        let backend = if let Some(dir) = data_dir {
            backend.with_asdf_data_dir(dir)
        } else {
            backend
        };
        Arc::new(backend)
    }

    fn create_manager_for_wsl(
        &self,
        distro: String,
        backend_path: String,
    ) -> Arc<dyn VersionManager> {
        Arc::new(AsdfBackend::with_wsl(distro, backend_path).with_in_path(false))
    }

    fn wsl_search_paths(&self) -> &'static [&'static str] {
        &[
            "$HOME/.asdf/bin/asdf",
            "$HOME/.local/bin/asdf",
            "$HOME/go/bin/asdf",
            "/usr/local/bin/asdf",
            "/usr/bin/asdf",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::AsdfProvider;

    versi_backend::provider_tests! {
        provider: AsdfProvider::new(reqwest::Client::new()),
        binary_name: "asdf",
        metadata: {
            name: "asdf",
            display_name: "asdf (asdf-nodejs)",
            shell_config_marker: "ASDF_DATA_DIR",
            shell_config_label: "asdf (asdf-nodejs)",
        },
        create_manager: {
            path: "/opt/homebrew/bin/asdf",
            version: "0.18.0",
            data_dir: "/tmp/asdf-data",
        },
        wsl_binary_path: "/usr/bin/asdf",
    }
}
