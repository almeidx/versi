use versi_asdf::AsdfProvider;
use versi_backend::{BackendDetection, BackendProvider};

fn integration_enabled() -> bool {
    std::env::var_os("VERSI_IT_ASDF").is_some()
}

#[tokio::test]
#[ignore = "requires a real asdf installation with nodejs plugin"]
async fn asdf_lists_versions_in_real_environment() {
    if !integration_enabled() {
        eprintln!("Skipping: set VERSI_IT_ASDF=1 to run real asdf integration tests");
        return;
    }

    let provider = AsdfProvider::new();
    let detection = provider.detect().await;
    assert!(
        detection.found,
        "asdf integration test requires asdf and nodejs plugin"
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
