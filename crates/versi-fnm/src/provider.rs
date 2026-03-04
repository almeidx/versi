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
    use std::collections::HashSet;
    use std::path::PathBuf;

    use versi_backend::{BackendDetection, BackendProvider};

    use super::FnmProvider;

    #[test]
    fn provider_metadata_is_stable() {
        let provider = FnmProvider::new();

        assert_eq!(provider.name(), "fnm");
        assert_eq!(provider.display_name(), "fnm (Fast Node Manager)");
        assert_eq!(provider.shell_config_marker(), "fnm env");
        assert_eq!(provider.shell_config_label(), "fnm (Fast Node Manager)");
    }

    #[test]
    fn create_manager_uses_detected_path_and_data_dir() {
        let provider = FnmProvider::new();
        let detection = BackendDetection {
            found: true,
            path: Some(PathBuf::from("/opt/homebrew/bin/fnm")),
            version: Some("1.38.0".to_string()),
            in_path: true,
            data_dir: Some(PathBuf::from("/tmp/fnm-data")),
        };

        let manager = provider.create_manager(&detection);
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("/opt/homebrew/bin/fnm"));
        assert_eq!(info.version.as_deref(), Some("1.38.0"));
        assert_eq!(info.data_dir, Some(PathBuf::from("/tmp/fnm-data")));
        assert!(info.in_path);
    }

    #[test]
    fn create_manager_falls_back_to_fnm_binary_name() {
        let provider = FnmProvider::new();
        let detection = BackendDetection {
            found: false,
            path: None,
            version: None,
            in_path: false,
            data_dir: None,
        };

        let manager = provider.create_manager(&detection);
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("fnm"));
        assert!(!info.in_path);
    }

    #[test]
    fn create_wsl_manager_uses_wsl_binary_path() {
        let provider = FnmProvider::new();

        let manager =
            provider.create_manager_for_wsl("Ubuntu".to_string(), "/usr/bin/fnm".to_string());
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("/usr/bin/fnm"));
        assert!(!info.in_path);
    }

    #[test]
    fn wsl_search_paths_are_unique() {
        let provider = FnmProvider::new();
        let paths = provider.wsl_search_paths();
        let unique_count = paths.iter().copied().collect::<HashSet<_>>().len();

        assert!(!paths.is_empty());
        assert_eq!(paths.len(), unique_count);
    }
}
