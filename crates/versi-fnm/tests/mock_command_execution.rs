#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use tempfile::tempdir;
use versi_backend::{BackendError, VersionManager};
use versi_fnm::FnmBackend;

fn write_executable(path: &Path, content: &str) {
    let tmp_path = path.with_file_name(format!(
        ".{}.tmp-{}",
        path.file_name()
            .expect("mock executable file name")
            .to_string_lossy(),
        std::process::id()
    ));

    // Write + chmod on a temporary path, then rename atomically to avoid
    // transient ETXTBSY ("Text file busy") when the file is executed on Linux.
    fs::write(&tmp_path, content).expect("write mock fnm executable");
    let perms = fs::Permissions::from_mode(0o755);
    fs::set_permissions(&tmp_path, perms).expect("set mock fnm executable permissions");
    fs::rename(&tmp_path, path).expect("atomically publish mock fnm executable");
}

fn mock_fnm_script(log_path: &Path) -> String {
    let log_path = log_path.display();
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
log_file="{log_path}"
cmd="${{1:-}}"
case "$cmd" in
  list)
    cat <<'OUT'
* v20.11.0 default
* v18.19.1
* system
OUT
    ;;
  list-remote)
    if [[ "${{2:-}}" == "--lts" ]]; then
      cat <<'OUT'
v20.11.1 (Iron)
v18.20.4 (Hydrogen)
OUT
    else
      cat <<'OUT'
v22.1.0
v20.11.1 (Iron)
OUT
    fi
    ;;
  current)
    echo "v20.11.0"
    ;;
  install)
    version="${{2:-}}"
    if [[ "$version" == "fail" ]]; then
      echo "install failed intentionally" >&2
      exit 17
    fi
    echo "install $version" >> "$log_file"
    ;;
  uninstall)
    echo "uninstall ${{2:-}}" >> "$log_file"
    ;;
  default)
    echo "default ${{2:-}}" >> "$log_file"
    ;;
  use)
    echo "use ${{2:-}}" >> "$log_file"
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
async fn fnm_backend_executes_mock_commands_and_parses_output() {
    let temp_dir = tempdir().expect("create temp dir");
    let fnm_path = temp_dir.path().join("fnm");
    let log_path = temp_dir.path().join("fnm.log");

    write_executable(&fnm_path, &mock_fnm_script(&log_path));

    let backend = FnmBackend::new(fnm_path, Some("test".to_string()), None);

    let installed = backend.list_installed().await.expect("list installed");
    assert_eq!(installed.len(), 2);
    assert!(installed[0].is_default);
    assert_eq!(installed[0].version.to_string(), "v20.11.0");
    assert_eq!(installed[1].version.to_string(), "v18.19.1");

    let remote = backend.list_remote().await.expect("list remote");
    assert_eq!(remote.len(), 2);
    assert_eq!(remote[0].version.to_string(), "v22.1.0");
    assert_eq!(remote[1].lts_codename.as_deref(), Some("Iron"));

    let lts = backend.list_remote_lts().await.expect("list remote lts");
    assert_eq!(lts.len(), 2);
    assert_eq!(lts[0].lts_codename.as_deref(), Some("Iron"));
    assert_eq!(lts[1].lts_codename.as_deref(), Some("Hydrogen"));

    let current = backend.current_version().await.expect("current version");
    assert_eq!(
        current.expect("current version should exist").to_string(),
        "v20.11.0"
    );

    backend.install("v20.11.1").await.expect("install");
    backend.uninstall("v18.19.1").await.expect("uninstall");
    backend.set_default("v20.11.1").await.expect("set default");
    backend.use_version("v20.11.1").await.expect("use version");

    let log = fs::read_to_string(log_path).expect("read command log");
    assert!(log.contains("install v20.11.1"));
    assert!(log.contains("uninstall v18.19.1"));
    assert!(log.contains("default v20.11.1"));
    assert!(log.contains("use v20.11.1"));
}

#[tokio::test]
async fn fnm_backend_surfaces_command_failures() {
    let temp_dir = tempdir().expect("create temp dir");
    let fnm_path = temp_dir.path().join("fnm");
    let log_path = temp_dir.path().join("fnm.log");
    write_executable(&fnm_path, &mock_fnm_script(&log_path));

    let backend = FnmBackend::new(fnm_path, Some("test".to_string()), None);
    let result = backend.install("fail").await;

    match &result {
        Err(BackendError::CommandFailed { stderr })
            if stderr.contains("install failed intentionally") => {}
        other => panic!(
            "expected Err(CommandFailed) containing 'install failed intentionally', got: {other:?}"
        ),
    }
}
