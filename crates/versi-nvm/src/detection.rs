use std::path::PathBuf;
use tokio::process::Command;

use versi_platform::HideWindow;

use crate::client::{NvmClient, NvmEnvironment};

#[derive(Debug, Clone)]
pub struct NvmDetection {
    pub found: bool,
    pub nvm_dir: Option<PathBuf>,
    pub nvm_exe: Option<PathBuf>,
    pub version: Option<String>,
    pub variant: NvmVariant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NvmVariant {
    Unix,
    Windows,
    NotFound,
}

pub async fn detect_nvm() -> NvmDetection {
    if let Some(detection) = detect_unix_nvm().await {
        return detection;
    }

    if let Some(detection) = detect_windows_nvm().await {
        return detection;
    }

    NvmDetection {
        found: false,
        nvm_dir: None,
        nvm_exe: None,
        version: None,
        variant: NvmVariant::NotFound,
    }
}

async fn detect_unix_nvm() -> Option<NvmDetection> {
    let nvm_dir = find_unix_nvm_dir()?;

    let nvm_sh = nvm_dir.join("nvm.sh");
    if !nvm_sh.exists() {
        return None;
    }

    let client = NvmClient::unix(nvm_dir.clone());
    let version = client.version().await.ok();

    Some(NvmDetection {
        found: true,
        nvm_dir: Some(nvm_dir),
        nvm_exe: None,
        version,
        variant: NvmVariant::Unix,
    })
}

fn find_unix_nvm_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("NVM_DIR") {
        let path = PathBuf::from(&dir);
        if path.join("nvm.sh").exists() {
            return Some(path);
        }
    }

    if let Some(home) = dirs::home_dir() {
        let default = home.join(".nvm");
        if default.join("nvm.sh").exists() {
            return Some(default);
        }
    }

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg).join("nvm");
        if path.join("nvm.sh").exists() {
            return Some(path);
        }
    }

    None
}

async fn detect_windows_nvm() -> Option<NvmDetection> {
    if let Ok(path) = which::which("nvm") {
        let version = get_windows_nvm_version(&path).await;
        return Some(NvmDetection {
            found: true,
            nvm_dir: None,
            nvm_exe: Some(path),
            version,
            variant: NvmVariant::Windows,
        });
    }

    let candidates = get_windows_nvm_paths();
    for path in candidates {
        if path.exists() {
            let version = get_windows_nvm_version(&path).await;
            return Some(NvmDetection {
                found: true,
                nvm_dir: None,
                nvm_exe: Some(path),
                version,
                variant: NvmVariant::Windows,
            });
        }
    }

    None
}

fn get_windows_nvm_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(appdata) = std::env::var("APPDATA") {
        paths.push(PathBuf::from(&appdata).join("nvm").join("nvm.exe"));
    }

    if let Ok(pf) = std::env::var("ProgramFiles") {
        paths.push(PathBuf::from(&pf).join("nvm").join("nvm.exe"));
    }

    paths
}

async fn get_windows_nvm_version(path: &PathBuf) -> Option<String> {
    let output = Command::new(path)
        .arg("version")
        .hide_window()
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Some(stdout.trim().to_string())
    } else {
        None
    }
}

pub fn detect_nvm_environment(detection: &NvmDetection) -> Option<NvmEnvironment> {
    match detection.variant {
        NvmVariant::Unix => detection.nvm_dir.as_ref().map(|dir| NvmEnvironment::Unix {
            nvm_dir: dir.clone(),
        }),
        NvmVariant::Windows => detection
            .nvm_exe
            .as_ref()
            .map(|exe| NvmEnvironment::Windows {
                nvm_exe: exe.clone(),
            }),
        NvmVariant::NotFound => None,
    }
}

pub async fn install_nvm() -> Result<(), crate::NvmError> {
    #[cfg(unix)]
    {
        let status = Command::new("bash")
            .args([
                "-c",
                "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/master/install.sh | bash",
            ])
            .hide_window()
            .status()
            .await?;

        if status.success() {
            Ok(())
        } else {
            Err(crate::NvmError::InstallFailed(
                "nvm installation script failed".to_string(),
            ))
        }
    }

    #[cfg(windows)]
    {
        Err(crate::NvmError::InstallFailed(
            "Automatic nvm-windows installation is not supported. Please install manually from https://github.com/coreybutler/nvm-windows/releases".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn unix_variant_maps_to_unix_environment() {
        let detection = NvmDetection {
            found: true,
            nvm_dir: Some(PathBuf::from("/home/user/.nvm")),
            nvm_exe: None,
            version: Some("0.40.1".to_string()),
            variant: NvmVariant::Unix,
        };
        let env = detect_nvm_environment(&detection).unwrap();
        assert!(
            matches!(env, NvmEnvironment::Unix { nvm_dir } if nvm_dir == Path::new("/home/user/.nvm"))
        );
    }

    #[test]
    fn windows_variant_maps_to_windows_environment() {
        let detection = NvmDetection {
            found: true,
            nvm_dir: None,
            nvm_exe: Some(PathBuf::from("C:\\nvm\\nvm.exe")),
            version: Some("1.1.12".to_string()),
            variant: NvmVariant::Windows,
        };
        let env = detect_nvm_environment(&detection).unwrap();
        assert!(
            matches!(env, NvmEnvironment::Windows { nvm_exe } if nvm_exe == Path::new("C:\\nvm\\nvm.exe"))
        );
    }

    #[test]
    fn not_found_variant_returns_none() {
        let detection = NvmDetection {
            found: false,
            nvm_dir: None,
            nvm_exe: None,
            version: None,
            variant: NvmVariant::NotFound,
        };
        assert!(detect_nvm_environment(&detection).is_none());
    }

    #[test]
    fn unix_with_missing_nvm_dir_returns_none() {
        let detection = NvmDetection {
            found: true,
            nvm_dir: None,
            nvm_exe: None,
            version: Some("0.40.1".to_string()),
            variant: NvmVariant::Unix,
        };
        assert!(detect_nvm_environment(&detection).is_none());
    }
}
