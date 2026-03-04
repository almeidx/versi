use std::sync::Arc;

use async_trait::async_trait;

use versi_backend::{
    BackendDetection, BackendError, BackendProvider, BackendUpdate, VersionManager,
};

use crate::backend::VoltaBackend;
use crate::detection::{detect_volta, detect_volta_home, install_volta};
use crate::update::check_for_volta_update;

pub struct VoltaProvider {
    http_client: reqwest::Client,
}

impl VoltaProvider {
    #[must_use]
    pub fn new(http_client: reqwest::Client) -> Self {
        Self { http_client }
    }
}

#[async_trait]
impl BackendProvider for VoltaProvider {
    fn name(&self) -> &'static str {
        "volta"
    }

    fn display_name(&self) -> &'static str {
        "Volta"
    }

    fn shell_config_marker(&self) -> &'static str {
        "VOLTA_HOME"
    }

    fn shell_config_label(&self) -> &'static str {
        "Volta"
    }

    async fn detect(&self) -> BackendDetection {
        detect_volta().await
    }

    async fn install_backend(&self) -> Result<(), BackendError> {
        install_volta().await
    }

    async fn check_for_update(
        &self,
        client: &reqwest::Client,
        current_version: &str,
        _detection: &BackendDetection,
    ) -> Result<Option<BackendUpdate>, BackendError> {
        check_for_volta_update(client, current_version).await
    }

    fn create_manager(&self, detection: &BackendDetection) -> Arc<dyn VersionManager> {
        let path = detection
            .path
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("volta"));
        let volta_home = detection.data_dir.clone().or_else(detect_volta_home);
        Arc::new(
            VoltaBackend::new(
                path,
                detection.version.clone(),
                volta_home,
                self.http_client.clone(),
            )
            .with_in_path(detection.in_path),
        )
    }

    fn create_manager_for_wsl(
        &self,
        distro: String,
        backend_path: String,
    ) -> Arc<dyn VersionManager> {
        Arc::new(
            VoltaBackend::with_wsl(distro, backend_path, self.http_client.clone())
                .with_in_path(false),
        )
    }

    fn wsl_search_paths(&self) -> &'static [&'static str] {
        &[
            "$HOME/.volta/bin/volta",
            "$HOME/.volta/bin/volta.exe",
            "$HOME/.cargo/bin/volta",
            "/usr/local/bin/volta",
            "/usr/bin/volta",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::VoltaProvider;

    versi_backend::provider_tests! {
        provider: VoltaProvider::new(reqwest::Client::new()),
        binary_name: "volta",
        metadata: {
            name: "volta",
            display_name: "Volta",
            shell_config_marker: "VOLTA_HOME",
            shell_config_label: "Volta",
        },
        create_manager: {
            path: "/usr/local/bin/volta",
            version: "2.0.2",
            data_dir: "/home/user/.volta",
        },
        wsl_binary_path: "/usr/bin/volta",
    }
}
