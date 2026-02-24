#![cfg(unix)]

use std::fs;
use std::path::Path;

use tempfile::tempdir;
use versi_backend::{BackendError, VersionManager};
use versi_nvm::{NvmBackend, NvmClient};

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).expect("write mock nvm script");
}

fn mock_nvm_sh(log_path: &Path) -> String {
    let log_path = log_path.display();
    format!(
        r#"#!/usr/bin/env bash
nvm() {{
  local cmd="${{1:-}}"
  shift || true
  case "$cmd" in
    list)
      cat <<'OUT'
->     v20.11.0
       v18.19.1
default -> 20 (-> v20.11.0)
OUT
      ;;
    ls-remote)
      if [[ "${{1:-}}" == "--lts" ]]; then
        cat <<'OUT'
        v20.11.1   (Latest LTS: Iron)
        v18.20.4   (LTS: Hydrogen)
OUT
      else
        cat <<'OUT'
        v22.1.0
        v20.11.1   (Latest LTS: Iron)
OUT
      fi
      ;;
    current)
      echo "v20.11.0"
      ;;
    alias)
      if [[ "${{1:-}}" == "default" && -n "${{2:-}}" ]]; then
        echo "alias default $2" >> "{log_path}"
        return 0
      fi
      if [[ "${{1:-}}" == "default" ]]; then
        echo "default -> 20 (-> v20.11.0)"
        return 0
      fi
      echo "unsupported alias invocation" >&2
      return 1
      ;;
    install)
      if [[ "${{1:-}}" == "fail" ]]; then
        echo "install failed intentionally" >&2
        return 42
      fi
      echo "install $1" >> "{log_path}"
      ;;
    uninstall)
      echo "uninstall $1" >> "{log_path}"
      ;;
    use)
      echo "use $1" >> "{log_path}"
      ;;
    --version)
      echo "0.40.1"
      ;;
    *)
      echo "unknown nvm command: $cmd" >&2
      return 127
      ;;
  esac
}}
"#
    )
}

#[tokio::test]
async fn nvm_backend_executes_mock_commands_and_parses_output() {
    let temp_dir = tempdir().expect("create temp dir");
    let nvm_dir = temp_dir.path().join(".nvm");
    fs::create_dir_all(&nvm_dir).expect("create mock nvm dir");
    let nvm_sh_path = nvm_dir.join("nvm.sh");
    let log_path = temp_dir.path().join("nvm.log");
    write_file(&nvm_sh_path, &mock_nvm_sh(&log_path));

    let client = NvmClient::unix(nvm_dir);
    let backend = NvmBackend::new(client, Some("0.40.1".to_string()));

    let installed = backend.list_installed().await.expect("list installed");
    assert_eq!(installed.len(), 2);
    assert_eq!(installed[0].version.to_string(), "v20.11.0");
    assert!(installed[0].is_default);

    let remote = backend.list_remote().await.expect("list remote");
    assert_eq!(remote.len(), 2);
    assert_eq!(remote[0].version.to_string(), "v22.1.0");

    let remote_lts = backend.list_remote_lts().await.expect("list remote lts");
    assert_eq!(remote_lts.len(), 2);
    assert!(
        remote_lts
            .iter()
            .all(|version| version.lts_codename.is_some())
    );

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
    backend.uninstall("v18.19.1").await.expect("uninstall");
    backend.set_default("v20.11.1").await.expect("set default");
    backend.use_version("v20.11.1").await.expect("use version");

    let log = fs::read_to_string(log_path).expect("read command log");
    assert!(log.contains("install v20.11.1"));
    assert!(log.contains("uninstall v18.19.1"));
    assert!(log.contains("alias default v20.11.1"));
    assert!(log.contains("use v20.11.1"));
}

#[tokio::test]
async fn nvm_backend_surfaces_command_failures() {
    let temp_dir = tempdir().expect("create temp dir");
    let nvm_dir = temp_dir.path().join(".nvm");
    fs::create_dir_all(&nvm_dir).expect("create mock nvm dir");
    let nvm_sh_path = nvm_dir.join("nvm.sh");
    let log_path = temp_dir.path().join("nvm.log");
    write_file(&nvm_sh_path, &mock_nvm_sh(&log_path));

    let client = NvmClient::unix(nvm_dir);
    let backend = NvmBackend::new(client, Some("0.40.1".to_string()));
    let result = backend.install("fail").await;

    assert!(matches!(
        result,
        Err(BackendError::CommandFailed { ref stderr })
            if stderr.contains("install failed intentionally")
    ));
}
