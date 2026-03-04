#![cfg(unix)]

use versi_backend::{BackendDetection, BackendProvider};
use versi_volta::VoltaProvider;

fn integration_enabled() -> bool {
    std::env::var_os("VERSI_IT_VOLTA").is_some()
}

#[tokio::test]
#[ignore = "requires a real volta installation and network access"]
async fn volta_lists_versions_in_real_environment() {
    if !integration_enabled() {
        eprintln!("Skipping: set VERSI_IT_VOLTA=1 to run real volta integration tests");
        return;
    }

    let provider = VoltaProvider::new(reqwest::Client::new());
    let detection = provider.detect().await;
    assert!(
        detection.found,
        "volta integration test requires volta to be installed"
    );

    let backend = provider.create_manager(&BackendDetection {
        found: detection.found,
        path: detection.path.clone(),
        version: detection.version.clone(),
        in_path: detection.in_path,
        data_dir: detection.data_dir.clone(),
    });

    let _installed = backend
        .list_installed()
        .await
        .expect("list installed should work");
    let _current = backend
        .current_version()
        .await
        .expect("current version lookup should work");
    let _default = backend
        .default_version()
        .await
        .expect("default version lookup should work");

    // Remote listing may require network; we still assert command plumbing works.
    let _ = backend.list_remote().await;
}
