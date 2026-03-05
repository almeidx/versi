#![cfg(unix)]

use std::fs;
use std::path::Path;

use tempfile::tempdir;
use versi_asdf::AsdfBackend;
use versi_backend::{BackendError, VersionManager, write_mock_executable};

fn mock_asdf_script(log_path: &Path) -> String {
    let log_path = log_path.display();
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
log_file="{log_path}"
cmd="${{1:-}}"
case "$cmd" in
  list)
    if [[ "${{2:-}}" == "nodejs" && -z "${{3:-}}" ]]; then
      cat <<'OUT'
  20.11.0
 *18.19.1
OUT
      exit 0
    fi
    if [[ "${{2:-}}" == "all" && "${{3:-}}" == "nodejs" ]]; then
      cat <<'OUT'
24.1.0
22.11.0
20.19.0
OUT
      exit 0
    fi
    echo "unsupported list invocation" >&2
    exit 2
    ;;
  current)
    echo "nodejs 20.11.0 $HOME/.tool-versions true"
    ;;
  install)
    version="${{3:-}}"
    if [[ "$version" == "fail" ]]; then
      echo "install failed intentionally" >&2
      exit 17
    fi
    echo "install nodejs $version" >> "$log_file"
    ;;
  uninstall)
    echo "uninstall nodejs ${{3:-}}" >> "$log_file"
    ;;
  set)
    echo "set ${{2:-}} ${{3:-}} ${{4:-}}" >> "$log_file"
    ;;
  --version)
    echo "asdf 0.18.0"
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
async fn asdf_backend_executes_mock_commands_and_parses_output() {
    let temp_dir = tempdir().expect("create temp dir");
    let asdf_path = temp_dir.path().join("asdf");
    let log_path = temp_dir.path().join("asdf.log");

    write_mock_executable!(&asdf_path, &mock_asdf_script(&log_path));

    let backend = AsdfBackend::new(asdf_path, Some("0.18.0".to_string()), None);

    let installed = backend.list_installed().await.expect("list installed");
    assert_eq!(installed.len(), 2);
    assert_eq!(installed[0].version.to_string(), "v20.11.0");
    assert!(installed[0].is_default);

    let remote = backend.list_remote().await.expect("list remote");
    assert_eq!(remote.len(), 3);
    assert_eq!(remote[0].version.to_string(), "v24.1.0");

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

    backend.install("v20.19.0").await.expect("install");
    backend.uninstall("v18.19.1").await.expect("uninstall");
    backend.set_default("v20.19.0").await.expect("set default");

    let log = fs::read_to_string(log_path).expect("read command log");
    assert!(log.contains("install nodejs 20.19.0"));
    assert!(log.contains("uninstall nodejs 18.19.1"));
    assert!(log.contains("set -u nodejs 20.19.0"));
}

#[tokio::test]
async fn asdf_backend_surfaces_command_failures() {
    let temp_dir = tempdir().expect("create temp dir");
    let asdf_path = temp_dir.path().join("asdf");
    let log_path = temp_dir.path().join("asdf.log");
    write_mock_executable!(&asdf_path, &mock_asdf_script(&log_path));

    let backend = AsdfBackend::new(asdf_path, Some("0.18.0".to_string()), None);
    let result = backend.install("fail").await;

    assert!(matches!(
        result,
        Err(BackendError::CommandFailed { ref stderr })
            if stderr.contains("install failed intentionally")
    ));
}
