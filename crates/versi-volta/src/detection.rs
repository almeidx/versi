use std::path::{Path, PathBuf};

use tokio::process::Command;
use which::which;

use versi_backend::BackendError;
use versi_core::HideWindow;
#[cfg(unix)]
use versi_core::download_install_script_unverified;
use versi_core::get_cli_version;
#[cfg(unix)]
use versi_core::temp_script_path;

#[cfg(unix)]
const VOLTA_INSTALL_SCRIPT_URL: &str = "https://get.volta.sh";

#[derive(Debug, Clone)]
pub struct VoltaDetection {
    pub found: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub in_path: bool,
    pub volta_home: Option<PathBuf>,
}

pub(crate) async fn detect_volta() -> VoltaDetection {
    let volta_home = detect_volta_home();

    if let Some(home) = &volta_home {
        let path = volta_home_binary_path(home);
        if path.exists() {
            let version = get_volta_version(&path).await;
            return VoltaDetection {
                found: true,
                path: Some(path),
                version,
                in_path: false,
                volta_home,
            };
        }
    }

    if let Ok(path) = which("volta") {
        let version = get_volta_version(&path).await;
        return VoltaDetection {
            found: true,
            path: Some(path),
            version,
            in_path: true,
            volta_home,
        };
    }

    for path in get_common_volta_paths() {
        if path.exists() {
            let version = get_volta_version(&path).await;
            return VoltaDetection {
                found: true,
                path: Some(path),
                version,
                in_path: false,
                volta_home,
            };
        }
    }

    VoltaDetection {
        found: false,
        path: None,
        version: None,
        in_path: false,
        volta_home,
    }
}

pub(crate) fn detect_volta_home() -> Option<PathBuf> {
    let env_dir = std::env::var("VOLTA_HOME").ok().map(PathBuf::from);
    select_volta_home(env_dir, get_volta_home_candidates())
}

fn select_volta_home(env_dir: Option<PathBuf>, candidates: Vec<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = env_dir.filter(|path| path.exists()) {
        return Some(path);
    }

    candidates.into_iter().find(|candidate| candidate.exists())
}

fn get_volta_home_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".volta"));
    }

    #[cfg(windows)]
    if let Some(local_data) = dirs::data_local_dir() {
        candidates.push(local_data.join("Volta"));
    }

    candidates
}

fn get_common_volta_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".volta").join("bin").join("volta"));
        paths.push(home.join(".volta").join("bin").join("volta.exe"));
    }

    #[cfg(unix)]
    {
        paths.push(PathBuf::from("/usr/local/bin/volta"));
        paths.push(PathBuf::from("/usr/bin/volta"));
    }

    #[cfg(windows)]
    {
        if let Some(local_data) = dirs::data_local_dir() {
            paths.push(local_data.join("Volta").join("bin").join("volta.exe"));
            paths.push(local_data.join("Volta").join("volta.exe"));
        }

        if let Ok(program_files) = std::env::var("ProgramFiles") {
            paths.push(
                PathBuf::from(&program_files)
                    .join("Volta")
                    .join("volta.exe"),
            );
            paths.push(
                PathBuf::from(&program_files)
                    .join("Volta")
                    .join("bin")
                    .join("volta.exe"),
            );
        }
    }

    paths
}

fn volta_home_binary_path(volta_home: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        volta_home.join("bin").join("volta.exe")
    }

    #[cfg(not(windows))]
    {
        volta_home.join("bin").join("volta")
    }
}

async fn get_volta_version(path: &Path) -> Option<String> {
    get_cli_version(path, "volta ").await
}

pub(crate) async fn install_volta() -> Result<(), BackendError> {
    #[cfg(unix)]
    {
        let script_path = temp_script_path("volta-install", "sh");
        let result = async {
            download_install_script(VOLTA_INSTALL_SCRIPT_URL, &script_path).await?;
            Command::new("bash")
                .arg(&script_path)
                .hide_window()
                .status()
                .await
                .map_err(BackendError::from)
        }
        .await;
        let _ = tokio::fs::remove_file(&script_path).await;
        let status = result?;

        if status.success() {
            Ok(())
        } else {
            Err(BackendError::install_failed(
                "run installer script",
                "volta installation script failed",
            ))
        }
    }

    #[cfg(windows)]
    {
        let status = Command::new("winget")
            .args([
                "install",
                "--id",
                "Volta.Volta",
                "--source",
                "winget",
                "--silent",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ])
            .hide_window()
            .status()
            .await
            .map_err(BackendError::from)?;

        if status.success() {
            Ok(())
        } else {
            Err(BackendError::install_failed(
                "run installer command",
                "volta installation command failed",
            ))
        }
    }
}

#[cfg(unix)]
async fn download_install_script(
    url: &str,
    path: &std::path::Path,
) -> Result<(), versi_backend::BackendError> {
    download_install_script_unverified(url, path)
        .await
        .map_err(|error| {
            versi_backend::BackendError::install_failed(
                "download installer script",
                format!("failed to download installer script: {error}"),
            )
        })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{get_common_volta_paths, select_volta_home};

    fn temp_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "versi-volta-detection-test-{}-{nonce}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn select_volta_home_prefers_existing_env_dir() {
        let env_dir = temp_path("env");
        let fallback = temp_path("fallback");
        std::fs::create_dir_all(&env_dir).expect("create env dir");
        std::fs::create_dir_all(&fallback).expect("create fallback dir");

        let selected = select_volta_home(Some(env_dir.clone()), vec![fallback.clone()]);

        assert_eq!(selected, Some(env_dir.clone()));
        let _ = std::fs::remove_dir_all(env_dir);
        let _ = std::fs::remove_dir_all(fallback);
    }

    #[test]
    fn select_volta_home_falls_back_to_existing_candidate() {
        let fallback = temp_path("fallback");
        std::fs::create_dir_all(&fallback).expect("create fallback dir");

        let selected = select_volta_home(None, vec![fallback.clone()]);

        assert_eq!(selected, Some(fallback.clone()));
        let _ = std::fs::remove_dir_all(fallback);
    }

    #[test]
    fn select_volta_home_returns_none_when_nothing_exists() {
        let missing = temp_path("missing");

        let selected = select_volta_home(None, vec![missing]);

        assert!(selected.is_none());
    }

    #[test]
    fn common_paths_include_home_volta_candidates() {
        let paths = get_common_volta_paths();
        let Some(home) = dirs::home_dir() else {
            return;
        };

        assert!(paths.contains(&home.join(".volta").join("bin").join("volta")));
    }

    #[cfg(unix)]
    #[test]
    fn temp_script_path_uses_requested_extension() {
        let path = versi_core::temp_script_path("volta-install-test", "sh");
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();

        assert!(file_name.contains("volta-install-test-"));
        assert!(
            std::path::Path::new(file_name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sh"))
        );
    }
}
