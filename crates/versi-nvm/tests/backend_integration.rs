#![cfg(unix)]

use std::fs;
use std::path::Path;

use tempfile::tempdir;
use versi_backend::{BackendError, BackendProvider, NodeVersion, VersionManager};
use versi_nvm::{NvmBackend, NvmClient, NvmProvider};

fn integration_enabled() -> bool {
    std::env::var_os("VERSI_IT_NVM").is_some()
}

fn integration_version() -> String {
    std::env::var("VERSI_IT_NODE_VERSION").unwrap_or_else(|_| "v20.11.1".to_string())
}

fn copy_if_exists(source_dir: &Path, target_dir: &Path, name: &str) {
    let source = source_dir.join(name);
    if !source.exists() {
        return;
    }

    let target = target_dir.join(name);
    if source.is_dir() {
        copy_dir_recursive(&source, &target);
    } else {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("create parent directory while copying nvm files");
        }
        fs::copy(&source, &target).expect("copy nvm file");
    }
}

fn copy_dir_recursive(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("create target directory for recursive copy");
    for entry in fs::read_dir(source).expect("read source directory for recursive copy") {
        let entry = entry.expect("read source directory entry");
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path);
        } else {
            fs::copy(&source_path, &target_path).expect("copy source file");
        }
    }
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
        let _ = uninstall_version(backend, version).await;
    }
}

async fn uninstall_version(
    backend: &dyn VersionManager,
    version: &str,
) -> Result<(), BackendError> {
    match backend.uninstall(version).await {
        Ok(()) => Ok(()),
        Err(BackendError::CommandFailed { stderr })
            if stderr.contains("currently-active node version") =>
        {
            let _ = backend.use_version("system").await;
            let _ = backend.set_default("system").await;
            backend.uninstall(version).await
        }
        Err(error) => Err(error),
    }
}

#[tokio::test]
#[ignore = "requires a real nvm installation and network access"]
async fn nvm_install_set_default_and_uninstall_roundtrip() {
    if !integration_enabled() {
        eprintln!("Skipping: set VERSI_IT_NVM=1 to run real nvm integration tests");
        return;
    }

    let provider = NvmProvider::new();
    let detection = provider.detect().await;
    assert!(
        detection.found,
        "nvm integration test requires nvm to be installed"
    );

    let source_nvm_dir = detection
        .data_dir
        .expect("nvm integration test requires a Unix-style NVM_DIR");

    let temp_dir = tempdir().expect("create isolated nvm dir");
    let isolated_nvm_dir = temp_dir.path().join(".nvm");
    fs::create_dir_all(&isolated_nvm_dir).expect("create isolated nvm directory");

    // nvm requires its scripts and helper binaries. Copy only required entries.
    for entry_name in ["nvm.sh", "nvm-exec", "bash_completion", "alias"] {
        copy_if_exists(&source_nvm_dir, &isolated_nvm_dir, entry_name);
    }

    let backend: std::sync::Arc<dyn VersionManager> = std::sync::Arc::new(NvmBackend::new(
        NvmClient::unix(isolated_nvm_dir),
        detection.version,
    ));

    let version = integration_version();
    let expected_default: NodeVersion = version
        .parse()
        .expect("VERSI_IT_NODE_VERSION should be a valid Node version");

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

    uninstall_version(backend.as_ref(), &version)
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
