use tempfile::tempdir;
use versi_backend::{BackendProvider, NodeVersion, VersionManager};
use versi_fnm::FnmProvider;

fn integration_enabled() -> bool {
    std::env::var_os("VERSI_IT_FNM").is_some()
}

fn integration_version() -> String {
    std::env::var("VERSI_IT_NODE_VERSION").unwrap_or_else(|_| "v20.11.1".to_string())
}

async fn uninstall_if_present(backend: &dyn VersionManager, version: &str) {
    let Ok(target_version) = version.parse::<NodeVersion>() else {
        return;
    };

    let Ok(installed) = backend.list_installed().await else {
        return;
    };

    if installed
        .iter()
        .any(|installed| installed.version == target_version)
    {
        let _ = backend.uninstall(version).await;
    }
}

#[tokio::test]
#[ignore = "requires a real fnm installation and network access"]
async fn fnm_install_set_default_and_uninstall_roundtrip() {
    if !integration_enabled() {
        eprintln!("Skipping: set VERSI_IT_FNM=1 to run real fnm integration tests");
        return;
    }

    let provider = FnmProvider::new();
    let detection = provider.detect().await;
    assert!(
        detection.found,
        "fnm integration test requires fnm to be installed"
    );

    let version = integration_version();
    let expected_default: NodeVersion = version
        .parse()
        .expect("VERSI_IT_NODE_VERSION should be a valid Node version");

    let temp_dir = tempdir().expect("create isolated fnm dir");
    let detection_path = detection
        .path
        .expect("fnm detection should include executable path");
    let isolated = versi_fnm::FnmBackend::new(detection_path, detection.version, None)
        .with_fnm_dir(temp_dir.path().to_path_buf());
    let backend: std::sync::Arc<dyn VersionManager> = std::sync::Arc::new(isolated);

    uninstall_if_present(backend.as_ref(), &version).await;

    backend
        .install(&version)
        .await
        .expect("install requested Node version");

    let installed = backend.list_installed().await.expect("list installed");
    assert!(
        installed
            .iter()
            .any(|installed| installed.version == expected_default),
        "installed versions should include {version}, got: {:?}",
        installed
            .iter()
            .map(|installed| installed.version.to_string())
            .collect::<Vec<_>>()
    );

    backend
        .set_default(&version)
        .await
        .expect("set installed version as default");
    let default_version = backend
        .default_version()
        .await
        .expect("read default version");
    assert_eq!(default_version, Some(expected_default.clone()));

    backend
        .uninstall(&version)
        .await
        .expect("uninstall test Node version");

    let installed_after = backend
        .list_installed()
        .await
        .expect("list installed after");
    assert!(
        !installed_after
            .iter()
            .any(|installed| installed.version == expected_default),
        "version should be removed after uninstall, got: {:?}",
        installed_after
            .iter()
            .map(|installed| installed.version.to_string())
            .collect::<Vec<_>>()
    );
}
