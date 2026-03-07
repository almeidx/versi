#[cfg(unix)]
#[macro_export]
macro_rules! write_mock_executable {
    ($path:expr, $content:expr) => {{
        use std::fs;
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        let path: &std::path::Path = $path;
        let mut file = fs::File::create(path).expect("create mock executable");
        file.write_all($content.as_bytes())
            .expect("write mock executable");
        file.sync_all().expect("sync mock executable");
        drop(file);
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .expect("set mock executable permissions");
    }};
}

#[macro_export]
macro_rules! provider_tests {
    (
        provider: $provider_expr:expr,
        binary_name: $binary_name:expr,
        metadata: {
            name: $name:expr,
            display_name: $display_name:expr,
            shell_config_marker: $marker:expr,
            shell_config_label: $label:expr,
        },
        create_manager: {
            path: $detect_path:expr,
            version: $detect_version:expr,
            data_dir: $detect_data_dir:expr,
        },
        wsl_binary_path: $wsl_path:expr $(,)?
    ) => {
        #[test]
        fn provider_metadata_is_stable() {
            let provider = $provider_expr;

            assert_eq!($crate::BackendProvider::name(&provider), $name,);
            assert_eq!(
                $crate::BackendProvider::display_name(&provider),
                $display_name,
            );
            assert_eq!(
                $crate::BackendProvider::shell_config_marker(&provider),
                $marker,
            );
            assert_eq!(
                $crate::BackendProvider::shell_config_label(&provider),
                $label,
            );
        }

        #[test]
        fn create_manager_uses_detected_path_and_data_dir() {
            let provider = $provider_expr;
            let detection = $crate::BackendDetection {
                found: true,
                path: Some(std::path::PathBuf::from($detect_path)),
                version: Some($detect_version.to_string()),
                in_path: true,
                data_dir: Some(std::path::PathBuf::from($detect_data_dir)),
            };

            let manager = $crate::BackendProvider::create_manager(&provider, &detection);
            let info = $crate::VersionManager::backend_info(manager.as_ref());

            assert_eq!(info.path, std::path::PathBuf::from($detect_path));
            assert_eq!(info.version.as_deref(), Some($detect_version));
            assert_eq!(
                info.data_dir,
                Some(std::path::PathBuf::from($detect_data_dir)),
            );
            assert!(info.in_path);
        }

        #[test]
        fn create_manager_falls_back_to_binary_name() {
            let provider = $provider_expr;
            let detection = $crate::BackendDetection {
                found: false,
                path: None,
                version: None,
                in_path: false,
                data_dir: None,
            };

            let manager = $crate::BackendProvider::create_manager(&provider, &detection);
            let info = $crate::VersionManager::backend_info(manager.as_ref());

            assert_eq!(info.path, std::path::PathBuf::from($binary_name));
            assert!(!info.in_path);
        }

        #[test]
        fn create_wsl_manager_uses_wsl_binary_path() {
            let provider = $provider_expr;

            let manager = $crate::BackendProvider::create_manager_for_wsl(
                &provider,
                "Ubuntu".to_string(),
                $wsl_path.to_string(),
            );
            let info = $crate::VersionManager::backend_info(manager.as_ref());

            assert_eq!(info.path, std::path::PathBuf::from($wsl_path));
            assert!(!info.in_path);
        }

        #[test]
        fn wsl_search_paths_are_unique() {
            let provider = $provider_expr;
            let paths = $crate::BackendProvider::wsl_search_paths(&provider);
            let unique_count = paths
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len();

            assert!(!paths.is_empty());
            assert_eq!(paths.len(), unique_count);
        }
    };
}
