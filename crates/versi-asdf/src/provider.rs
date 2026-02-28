use std::sync::Arc;

use async_trait::async_trait;

use versi_backend::{
    BackendDetection, BackendError, BackendProvider, BackendUpdate, VersionManager,
};

use crate::backend::AsdfBackend;
use crate::detection::{detect_asdf, detect_asdf_data_dir, install_asdf};
use crate::update::check_for_asdf_update;

#[derive(Default)]
pub struct AsdfProvider;

impl AsdfProvider {
    #[must_use]
    pub fn new() -> Self {
        Self
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
        let detection = detect_asdf().await;
        BackendDetection {
            found: detection.found,
            path: detection.path,
            version: detection.version,
            in_path: detection.in_path,
            data_dir: detection.asdf_data_dir,
        }
    }

    async fn install_backend(&self) -> Result<(), BackendError> {
        install_asdf().await
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
    use std::collections::HashSet;
    use std::path::PathBuf;

    use versi_backend::{BackendDetection, BackendProvider};

    use super::AsdfProvider;

    #[test]
    fn provider_metadata_is_stable() {
        let provider = AsdfProvider::new();

        assert_eq!(provider.name(), "asdf");
        assert_eq!(provider.display_name(), "asdf (asdf-nodejs)");
        assert_eq!(provider.shell_config_marker(), "ASDF_DATA_DIR");
        assert_eq!(provider.shell_config_label(), "asdf (asdf-nodejs)");
    }

    #[test]
    fn create_manager_uses_detected_path_and_data_dir() {
        let provider = AsdfProvider::new();
        let detection = BackendDetection {
            found: true,
            path: Some(PathBuf::from("/opt/homebrew/bin/asdf")),
            version: Some("0.18.0".to_string()),
            in_path: true,
            data_dir: Some(PathBuf::from("/tmp/asdf-data")),
        };

        let manager = provider.create_manager(&detection);
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("/opt/homebrew/bin/asdf"));
        assert_eq!(info.version.as_deref(), Some("0.18.0"));
        assert_eq!(info.data_dir, Some(PathBuf::from("/tmp/asdf-data")));
        assert!(info.in_path);
    }

    #[test]
    fn create_manager_falls_back_to_asdf_binary_name() {
        let provider = AsdfProvider::new();
        let detection = BackendDetection {
            found: false,
            path: None,
            version: None,
            in_path: false,
            data_dir: None,
        };

        let manager = provider.create_manager(&detection);
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("asdf"));
        assert!(!info.in_path);
    }

    #[test]
    fn create_wsl_manager_uses_wsl_binary_path() {
        let provider = AsdfProvider::new();

        let manager =
            provider.create_manager_for_wsl("Ubuntu".to_string(), "/usr/bin/asdf".to_string());
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("/usr/bin/asdf"));
        assert!(!info.in_path);
    }

    #[test]
    fn wsl_search_paths_are_unique() {
        let provider = AsdfProvider::new();
        let paths = provider.wsl_search_paths();
        let unique_count = paths.iter().copied().collect::<HashSet<_>>().len();

        assert!(!paths.is_empty());
        assert_eq!(paths.len(), unique_count);
    }
}
