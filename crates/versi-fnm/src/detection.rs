use std::path::{Path, PathBuf};
#[cfg(windows)]
use tokio::process::Command;
use which::which;

use versi_backend::BackendDetection;
#[cfg(windows)]
use versi_backend::download_and_prepare_install_script;
use versi_core::get_cli_version;
#[cfg(windows)]
use versi_core::{HideWindow, temp_script_path};

const FNM_INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/Schniz/fnm/v1.38.1/.ci/install.sh";

pub(crate) async fn detect_fnm() -> BackendDetection {
    let data_dir = detect_fnm_dir();

    if let Ok(path) = which("fnm") {
        let version = get_fnm_version(&path).await;
        return BackendDetection {
            found: true,
            path: Some(path),
            version,
            in_path: true,
            data_dir,
        };
    }

    let common_paths = get_common_fnm_paths();

    for path in common_paths {
        if path.exists() {
            let version = get_fnm_version(&path).await;
            return BackendDetection {
                found: true,
                path: Some(path),
                version,
                in_path: false,
                data_dir,
            };
        }
    }

    BackendDetection {
        found: false,
        path: None,
        version: None,
        in_path: false,
        data_dir,
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
    {
        return versi_backend::run_unix_install_script(FNM_INSTALL_SCRIPT_URL, "fnm-install").await;
    }

    #[cfg(windows)]
    {
        let script_path = temp_script_path("fnm-install", "ps1");
        let result = async {
            download_and_prepare_install_script(FNM_INSTALL_SCRIPT_URL, &script_path).await?;
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
        let status = result?;

        if status.success() {
            Ok(())
        } else {
            Err(versi_backend::BackendError::install_failed(
                "run installer script",
                "fnm installation script failed",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{get_common_fnm_paths, select_fnm_dir};

    #[test]
    fn select_fnm_dir_prefers_existing_env_dir() {
        let env_dir = tempfile::tempdir().expect("create env dir");
        let candidate = tempfile::tempdir().expect("create candidate dir");
        std::fs::create_dir_all(candidate.path().join("node-versions"))
            .expect("create node-versions subdir");

        let selected = select_fnm_dir(
            Some(env_dir.path().to_path_buf()),
            vec![candidate.path().to_path_buf()],
        );

        assert_eq!(selected, Some(env_dir.path().to_path_buf()));
    }

    #[test]
    fn select_fnm_dir_prefers_node_versions_candidate() {
        let plain = tempfile::tempdir().expect("create plain candidate");
        let with_versions = tempfile::tempdir().expect("create node-versions candidate");
        std::fs::create_dir_all(with_versions.path().join("node-versions"))
            .expect("create node-versions subdir");

        let selected = select_fnm_dir(
            None,
            vec![
                plain.path().to_path_buf(),
                with_versions.path().to_path_buf(),
            ],
        );

        assert_eq!(selected, Some(with_versions.path().to_path_buf()));
    }

    #[test]
    fn select_fnm_dir_falls_back_to_existing_candidate() {
        let fallback = tempfile::tempdir().expect("create fallback candidate");

        let selected = select_fnm_dir(None, vec![fallback.path().to_path_buf()]);

        assert_eq!(selected, Some(fallback.path().to_path_buf()));
    }

    #[test]
    fn select_fnm_dir_returns_none_when_nothing_exists() {
        let base = tempfile::tempdir().expect("create temp dir");
        let missing = base.path().join("nonexistent");

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
