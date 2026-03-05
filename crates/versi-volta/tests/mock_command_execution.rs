#![cfg(unix)]

use std::fs;
use std::path::Path;

use tempfile::tempdir;
use versi_backend::{BackendError, VersionManager, write_mock_executable};
use versi_volta::VoltaBackend;

fn mock_volta_script(log_path: &Path) -> String {
    let log_path = log_path.display();
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
log_file="{log_path}"
cmd="${{1:-}}"
case "$cmd" in
  list)
    if [[ "${{2:-}}" == "node" && "${{3:-}}" == "--format" && "${{4:-}}" == "plain" ]]; then
      cat <<'OUT'
runtime node@20.11.0 (default)
runtime node@18.19.1
OUT
      exit 0
    fi
    if [[ "${{2:-}}" == "--current" && "${{3:-}}" == "node" && "${{4:-}}" == "--format" && "${{5:-}}" == "plain" ]]; then
      echo "runtime node@20.11.0 (current @ /tmp/project)"
      exit 0
    fi
    if [[ "${{2:-}}" == "--default" && "${{3:-}}" == "node" && "${{4:-}}" == "--format" && "${{5:-}}" == "plain" ]]; then
      echo "runtime node@20.11.0 (default)"
      exit 0
    fi
    echo "unsupported list invocation: $*" >&2
    exit 2
    ;;
  fetch)
    spec="${{2:-}}"
    if [[ "$spec" == "node@fail" ]]; then
      echo "fetch failed intentionally" >&2
      exit 17
    fi
    echo "fetch $spec" >> "$log_file"
    ;;
  install)
    echo "install ${{2:-}}" >> "$log_file"
    ;;
  --version)
    echo "2.0.2"
    ;;
  *)
    echo "unknown command: $*" >&2
    exit 1
    ;;
esac
"#
    )
}

#[tokio::test]
async fn volta_backend_executes_mock_commands_and_parses_output() {
    let temp_dir = tempdir().expect("create temp dir");
    let volta_path = temp_dir.path().join("volta");
    let log_path = temp_dir.path().join("volta.log");

    write_mock_executable!(&volta_path, &mock_volta_script(&log_path));

    let backend = VoltaBackend::new(
        volta_path,
        Some("2.0.2".to_string()),
        None,
        reqwest::Client::new(),
    );

    let installed = backend.list_installed().await.expect("list installed");
    assert_eq!(installed.len(), 2);
    assert_eq!(installed[0].version.to_string(), "v20.11.0");
    assert!(installed[0].is_default);

    let current = backend.current_version().await.expect("current version");
    assert_eq!(
        current.expect("current version should exist").to_string(),
        "v20.11.0"
    );

    let default_version = backend.default_version().await.expect("default version");
    assert_eq!(
        default_version
            .expect("default version should exist")
            .to_string(),
        "v20.11.0"
    );

    backend.install("v20.11.1").await.expect("install");
    backend.set_default("v20.11.1").await.expect("set default");

    let log = fs::read_to_string(log_path).expect("read command log");
    assert!(log.contains("fetch node@20.11.1"));
    assert!(log.contains("install node@20.11.1"));
}

#[tokio::test]
async fn volta_backend_surfaces_command_failures() {
    let temp_dir = tempdir().expect("create temp dir");
    let volta_path = temp_dir.path().join("volta");
    let log_path = temp_dir.path().join("volta.log");
    write_mock_executable!(&volta_path, &mock_volta_script(&log_path));

    let backend = VoltaBackend::new(
        volta_path,
        Some("2.0.2".to_string()),
        None,
        reqwest::Client::new(),
    );
    let result = backend.install("fail").await;

    assert!(matches!(
        result,
        Err(BackendError::CommandFailed { ref stderr })
            if stderr.contains("fetch failed intentionally")
    ));
}
