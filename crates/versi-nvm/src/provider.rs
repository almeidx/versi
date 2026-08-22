use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;

use versi_backend::{
    BackendDetection, BackendError, BackendProvider, BackendUpdate, VersionManager,
};

use crate::backend::NvmBackend;
use crate::client::{NvmClient, NvmEnvironment};
use crate::detection::{NvmVariant, detect_nvm, detect_nvm_environment, install_nvm};
use crate::update::check_for_nvm_update;

#[derive(Default)]
pub struct NvmProvider;

impl NvmProvider {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

fn variant_from_detection(detection: &BackendDetection) -> NvmVariant {
    if detection.data_dir.is_some() {
        NvmVariant::Unix
    } else if detection.path.is_some() {
        NvmVariant::Windows
    } else {
        NvmVariant::NotFound
    }
}

fn same_binary_path(path: &Path, which_path: &Path) -> bool {
    match (
        std::fs::canonicalize(path),
        std::fs::canonicalize(which_path),
    ) {
        (Ok(lhs), Ok(rhs)) => lhs == rhs,
        _ => {
            if cfg!(windows) {
                path.to_string_lossy()
                    .eq_ignore_ascii_case(&which_path.to_string_lossy())
            } else {
                path == which_path
            }
        }
    }
}

fn detection_in_path(detection: &crate::detection::NvmDetection) -> bool {
    match detection.variant {
        NvmVariant::Unix | NvmVariant::NotFound => false,
        NvmVariant::Windows => detection.nvm_exe.as_ref().is_some_and(|path| {
            which::which("nvm").is_ok_and(|which_path| same_binary_path(path, &which_path))
        }),
    }
}

#[async_trait]
impl BackendProvider for NvmProvider {
    fn name(&self) -> &'static str {
        "nvm"
    }

    fn display_name(&self) -> &'static str {
        "nvm (Node Version Manager)"
    }

    fn shell_config_marker(&self) -> &'static str {
        "NVM_DIR"
    }

    fn shell_config_label(&self) -> &'static str {
        "nvm (Node Version Manager)"
    }

    async fn detect(&self) -> BackendDetection {
        let detection = detect_nvm().await;
        let in_path = detection_in_path(&detection);
        let path = detection
            .nvm_dir
            .as_ref()
            .or(detection.nvm_exe.as_ref())
            .cloned();

        BackendDetection {
            found: detection.found,
            in_path,
            version: detection.version,
            data_dir: detection.nvm_dir,
            path,
        }
    }

    async fn install_backend(&self) -> Result<(), BackendError> {
        install_nvm().await
    }

    async fn check_for_update(
        &self,
        client: &reqwest::Client,
        current_version: &str,
        detection: &BackendDetection,
    ) -> Result<Option<BackendUpdate>, BackendError> {
        let variant = variant_from_detection(detection);
        check_for_nvm_update(client, current_version, &variant).await
    }

    fn create_manager(&self, detection: &BackendDetection) -> Arc<dyn VersionManager> {
        let variant = variant_from_detection(detection);
        let data_dir = detection.data_dir.clone();
        let version = detection.version.clone();

        let nvm_detection = crate::detection::NvmDetection {
            found: detection.found,
            nvm_dir: data_dir.clone(),
            nvm_exe: if variant == NvmVariant::Windows {
                detection.path.clone()
            } else {
                None
            },
            version: version.clone(),
            variant,
        };

        let environment =
            detect_nvm_environment(&nvm_detection).unwrap_or_else(|| NvmEnvironment::Unix {
                nvm_dir: data_dir
                    .or_else(|| detection.path.clone())
                    .unwrap_or_else(|| PathBuf::from("~/.nvm")),
            });

        let client = NvmClient { environment };

        Arc::new(NvmBackend::new(client, version).with_in_path(detection.in_path))
    }

    fn create_manager_for_wsl(
        &self,
        distro: String,
        backend_path: String,
    ) -> Arc<dyn VersionManager> {
        let nvm_dir = if backend_path.ends_with("nvm.sh") {
            backend_path
                .strip_suffix("/nvm.sh")
                .unwrap_or(&backend_path)
                .to_string()
        } else {
            backend_path
        };

        let client = NvmClient::wsl(distro, nvm_dir);
        Arc::new(NvmBackend::new(client, None).with_in_path(false))
    }

    fn wsl_search_paths(&self) -> &'static [&'static str] {
        &["$HOME/.nvm/nvm.sh"]
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use versi_backend::{BackendDetection, BackendProvider};

    use super::{NvmProvider, detection_in_path, variant_from_detection};
    use crate::detection::{NvmDetection, NvmVariant};

    #[test]
    fn variant_from_detection_unix_when_data_dir_set() {
        let detection = BackendDetection {
            found: true,
            path: Some(PathBuf::from("/home/user/.nvm")),
            version: Some("0.40.1".to_string()),
            in_path: true,
            data_dir: Some(PathBuf::from("/home/user/.nvm")),
        };
        assert_eq!(variant_from_detection(&detection), NvmVariant::Unix);
    }

    #[test]
    fn variant_from_detection_windows_when_only_path_set() {
        let detection = BackendDetection {
            found: true,
            path: Some(PathBuf::from("C:\\nvm\\nvm.exe")),
            version: Some("1.1.12".to_string()),
            in_path: true,
            data_dir: None,
        };
        assert_eq!(variant_from_detection(&detection), NvmVariant::Windows);
    }

    #[test]
    fn variant_from_detection_not_found_when_both_none() {
        let detection = BackendDetection {
            found: false,
            path: None,
            version: None,
            in_path: false,
            data_dir: None,
        };
        assert_eq!(variant_from_detection(&detection), NvmVariant::NotFound);
    }

    #[test]
    fn detection_in_path_is_false_for_unix_detection() {
        let detection = NvmDetection {
            found: true,
            nvm_dir: Some(PathBuf::from("/home/user/.nvm")),
            nvm_exe: None,
            version: Some("0.40.1".to_string()),
            variant: NvmVariant::Unix,
        };

        assert!(!detection_in_path(&detection));
    }

    #[test]
    fn detection_in_path_is_false_for_not_found_detection() {
        let detection = NvmDetection {
            found: false,
            nvm_dir: None,
            nvm_exe: None,
            version: None,
            variant: NvmVariant::NotFound,
        };

        assert!(!detection_in_path(&detection));
    }

    #[test]
    fn provider_metadata_is_stable() {
        let provider = NvmProvider::new();

        assert_eq!(provider.name(), "nvm");
        assert_eq!(provider.display_name(), "nvm (Node Version Manager)");
        assert_eq!(provider.shell_config_marker(), "NVM_DIR");
        assert_eq!(provider.shell_config_label(), "nvm (Node Version Manager)");
    }

    #[test]
    fn create_manager_uses_detection_data_dir_for_unix_fallback() {
        let provider = NvmProvider::new();
        let detection = BackendDetection {
            found: true,
            path: Some(PathBuf::from("/custom/nvm.sh")),
            version: Some("0.40.1".to_string()),
            in_path: true,
            data_dir: Some(PathBuf::from("/custom/.nvm")),
        };

        let manager = provider.create_manager(&detection);
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("/custom/.nvm/nvm.sh"));
        assert_eq!(info.data_dir, Some(PathBuf::from("/custom/.nvm")));
        assert_eq!(info.version.as_deref(), Some("0.40.1"));
        assert!(info.in_path);
    }

    #[test]
    fn create_wsl_manager_trims_nvm_script_suffix() {
        let provider = NvmProvider::new();

        let manager = provider
            .create_manager_for_wsl("Ubuntu".to_string(), "/home/user/.nvm/nvm.sh".to_string());

        assert_eq!(
            manager.backend_info().path,
            PathBuf::from("/home/user/.nvm/nvm.sh")
        );
        assert_eq!(
            manager.backend_info().data_dir,
            Some(PathBuf::from("/home/user/.nvm"))
        );
        assert!(!manager.backend_info().in_path);
    }

    #[test]
    fn wsl_search_paths_contains_default_nvm_script_path() {
        let provider = NvmProvider::new();

        assert_eq!(provider.wsl_search_paths(), vec!["$HOME/.nvm/nvm.sh"]);
    }

    #[test]
    fn wsl_search_paths_are_unique() {
        let provider = NvmProvider::new();
        let paths = provider.wsl_search_paths();
        let unique_count = paths
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len();

        assert!(!paths.is_empty());
        assert_eq!(paths.len(), unique_count);
    }

    #[test]
    fn create_manager_falls_back_when_detection_is_empty() {
        let provider = NvmProvider::new();
        let detection = BackendDetection {
            found: false,
            path: None,
            version: None,
            in_path: false,
            data_dir: None,
        };

        let manager = provider.create_manager(&detection);
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("~/.nvm/nvm.sh"));
        assert!(!info.in_path);
    }
}
