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
    use std::collections::HashSet;
    use std::path::PathBuf;

    use versi_backend::{BackendDetection, BackendProvider};

    use super::VoltaProvider;

    fn provider() -> VoltaProvider {
        VoltaProvider::new(reqwest::Client::new())
    }

    #[test]
    fn provider_metadata_is_stable() {
        let provider = provider();

        assert_eq!(provider.name(), "volta");
        assert_eq!(provider.display_name(), "Volta");
        assert_eq!(provider.shell_config_marker(), "VOLTA_HOME");
        assert_eq!(provider.shell_config_label(), "Volta");
    }

    #[test]
    fn create_manager_uses_detected_path_and_data_dir() {
        let provider = provider();
        let detection = BackendDetection {
            found: true,
            path: Some(PathBuf::from("/usr/local/bin/volta")),
            version: Some("2.0.2".to_string()),
            in_path: true,
            data_dir: Some(PathBuf::from("/home/user/.volta")),
        };

        let manager = provider.create_manager(&detection);
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("/usr/local/bin/volta"));
        assert_eq!(info.version.as_deref(), Some("2.0.2"));
        assert_eq!(info.data_dir, Some(PathBuf::from("/home/user/.volta")));
        assert!(info.in_path);
    }

    #[test]
    fn create_manager_falls_back_to_volta_binary_name() {
        let provider = provider();
        let detection = BackendDetection {
            found: false,
            path: None,
            version: None,
            in_path: false,
            data_dir: None,
        };

        let manager = provider.create_manager(&detection);
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("volta"));
        assert!(!info.in_path);
    }

    #[test]
    fn create_wsl_manager_uses_wsl_binary_path() {
        let provider = provider();
        let manager =
            provider.create_manager_for_wsl("Ubuntu".to_string(), "/usr/bin/volta".to_string());
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("/usr/bin/volta"));
        assert!(!info.in_path);
    }

    #[test]
    fn wsl_search_paths_are_unique() {
        let provider = provider();
        let paths = provider.wsl_search_paths();
        let unique_count = paths.iter().copied().collect::<HashSet<_>>().len();

        assert!(!paths.is_empty());
        assert_eq!(paths.len(), unique_count);
    }
}
