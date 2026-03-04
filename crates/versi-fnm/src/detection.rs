use std::path::{Path, PathBuf};
use tokio::process::Command;
use which::which;

use versi_core::{HideWindow, download_install_script, get_cli_version, temp_script_path};

const FNM_INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/Schniz/fnm/v1.38.1/.ci/install.sh";

#[derive(Debug, Clone)]
pub struct FnmDetection {
    pub found: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub in_path: bool,
    pub fnm_dir: Option<PathBuf>,
}

pub(crate) async fn detect_fnm() -> FnmDetection {
    let fnm_dir = detect_fnm_dir();

    if let Ok(path) = which("fnm") {
        let version = get_fnm_version(&path).await;
        return FnmDetection {
            found: true,
            path: Some(path),
            version,
            in_path: true,
            fnm_dir,
        };
    }

    let common_paths = get_common_fnm_paths();

    for path in common_paths {
        if path.exists() {
            let version = get_fnm_version(&path).await;
            return FnmDetection {
                found: true,
                path: Some(path),
                version,
                in_path: false,
                fnm_dir,
            };
        }
    }

    FnmDetection {
        found: false,
        path: None,
        version: None,
        in_path: false,
        fnm_dir,
    }
}

pub(crate) fn detect_fnm_dir() -> Option<PathBuf> {
    let env_dir = std::env::var("FNM_DIR").ok().map(PathBuf::from);
    select_fnm_dir(env_dir, get_fnm_dir_candidates())
}

fn select_fnm_dir(env_dir: Option<PathBuf>, candidates: Vec<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = env_dir.filter(|path| path.exists()) {
        return Some(path);
    }

    candidates
        .iter()
        .find(|candidate| candidate.exists() && candidate.join("node-versions").exists())
        .cloned()
        .or_else(|| candidates.into_iter().find(|candidate| candidate.exists()))
}

fn get_fnm_dir_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        paths.push(PathBuf::from(xdg_data).join("fnm"));
    }

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".local").join("share").join("fnm"));
        paths.push(home.join(".fnm"));
    }

    if let Some(data_dir) = dirs::data_local_dir() {
        paths.push(data_dir.join("fnm"));
    }

    paths
}

fn get_common_fnm_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".fnm").join("fnm"));
        paths.push(home.join(".local").join("bin").join("fnm"));
        paths.push(home.join(".cargo").join("bin").join("fnm"));

        #[cfg(target_os = "macos")]
        {
            paths.push(PathBuf::from("/opt/homebrew/bin/fnm"));
        }

        #[cfg(unix)]
        {
            paths.push(PathBuf::from("/usr/local/bin/fnm"));
            paths.push(PathBuf::from("/usr/bin/fnm"));
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(local_app_data) = dirs::data_local_dir() {
                paths.push(local_app_data.join("fnm").join("fnm.exe"));
            }
        }
    }

    paths
}

async fn get_fnm_version(path: &Path) -> Option<String> {
    get_cli_version(path, "fnm ").await
}

pub(crate) async fn install_fnm() -> Result<(), versi_backend::BackendError> {
    #[cfg(unix)]
    let status = {
        let script_path = temp_script_path("fnm-install", "sh");
        let result = async {
            download_and_prepare_script(FNM_INSTALL_SCRIPT_URL, &script_path).await?;
            Command::new("bash")
                .arg(&script_path)
                .hide_window()
                .status()
                .await
                .map_err(versi_backend::BackendError::from)
        }
        .await;
        let _ = tokio::fs::remove_file(&script_path).await;
        result?
    };

    #[cfg(windows)]
    let status = {
        let script_path = temp_script_path("fnm-install", "ps1");
        let result = async {
            download_and_prepare_script(FNM_INSTALL_SCRIPT_URL, &script_path).await?;
            Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    &script_path.to_string_lossy(),
                ])
                .hide_window()
                .status()
                .await
                .map_err(versi_backend::BackendError::from)
        }
        .await;
        let _ = tokio::fs::remove_file(&script_path).await;
        result?
    };

    if status.success() {
        Ok(())
    } else {
        Err(versi_backend::BackendError::install_failed(
            "run installer script",
            "fnm installation script failed",
        ))
    }
}

async fn download_and_prepare_script(
    url: &str,
    path: &std::path::Path,
) -> Result<(), versi_backend::BackendError> {
    download_install_script(url, path).await.map_err(|error| {
        versi_backend::BackendError::install_failed(
            "download installer script",
            format!("failed to download installer script: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{get_common_fnm_paths, select_fnm_dir};

    fn temp_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "versi-fnm-detection-test-{}-{nonce}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn select_fnm_dir_prefers_existing_env_dir() {
        let env_dir = temp_path("env");
        let candidate = temp_path("candidate");
        std::fs::create_dir_all(&env_dir).expect("create env dir");
        std::fs::create_dir_all(candidate.join("node-versions")).expect("create candidate dir");

        let selected = select_fnm_dir(Some(env_dir.clone()), vec![candidate.clone()]);

        assert_eq!(selected, Some(env_dir.clone()));
        let _ = std::fs::remove_dir_all(env_dir);
        let _ = std::fs::remove_dir_all(candidate);
    }

    #[test]
    fn select_fnm_dir_prefers_node_versions_candidate() {
        let plain = temp_path("plain");
        let with_versions = temp_path("with-node-versions");
        std::fs::create_dir_all(&plain).expect("create plain candidate");
        std::fs::create_dir_all(with_versions.join("node-versions"))
            .expect("create node-versions candidate");

        let selected = select_fnm_dir(None, vec![plain.clone(), with_versions.clone()]);

        assert_eq!(selected, Some(with_versions.clone()));
        let _ = std::fs::remove_dir_all(plain);
        let _ = std::fs::remove_dir_all(with_versions);
    }

    #[test]
    fn select_fnm_dir_falls_back_to_existing_candidate() {
        let fallback = temp_path("fallback");
        std::fs::create_dir_all(&fallback).expect("create fallback candidate");

        let selected = select_fnm_dir(None, vec![fallback.clone()]);

        assert_eq!(selected, Some(fallback.clone()));
        let _ = std::fs::remove_dir_all(fallback);
    }

    #[test]
    fn select_fnm_dir_returns_none_when_nothing_exists() {
        let missing = temp_path("missing");

        let selected = select_fnm_dir(None, vec![missing]);

        assert!(selected.is_none());
    }

    #[test]
    fn common_paths_include_expected_home_candidates() {
        let paths = get_common_fnm_paths();
        let Some(home) = dirs::home_dir() else {
            return;
        };

        assert!(paths.contains(&home.join(".fnm").join("fnm")));
        assert!(paths.contains(&home.join(".local").join("bin").join("fnm")));
        assert!(paths.contains(&home.join(".cargo").join("bin").join("fnm")));
    }
}
