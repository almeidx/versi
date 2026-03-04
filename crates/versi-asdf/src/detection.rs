use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use tokio::process::Command;
use which::which;

#[cfg(unix)]
use versi_core::GitHubRelease;
use versi_core::HideWindow;

#[cfg(unix)]
const ASDF_RELEASES_API: &str = "https://api.github.com/repos/asdf-vm/asdf/releases/latest";
#[cfg(unix)]
const ASDF_NODEJS_PLUGIN_URL: &str = "https://github.com/asdf-vm/asdf-nodejs.git";

#[cfg(any(test, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InstallerAttempt {
    label: &'static str,
    program: &'static str,
    args: &'static [&'static str],
}

#[cfg(any(test, windows))]
const ASDF_WINDOWS_INSTALL_ATTEMPTS: [InstallerAttempt; 3] = [
    InstallerAttempt {
        label: "winget",
        program: "winget",
        args: &[
            "install",
            "--id",
            "asdf-vm.asdf",
            "--source",
            "winget",
            "--silent",
            "--accept-source-agreements",
            "--accept-package-agreements",
        ],
    },
    InstallerAttempt {
        label: "scoop",
        program: "scoop",
        args: &["install", "asdf"],
    },
    InstallerAttempt {
        label: "choco",
        program: "choco",
        args: &["install", "asdf", "-y"],
    },
];

#[cfg(any(test, windows))]
fn asdf_windows_install_attempts() -> &'static [InstallerAttempt] {
    &ASDF_WINDOWS_INSTALL_ATTEMPTS
}

#[derive(Debug, Clone)]
pub struct AsdfDetection {
    pub found: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub in_path: bool,
    pub asdf_data_dir: Option<PathBuf>,
}

pub(crate) async fn detect_asdf() -> AsdfDetection {
    let asdf_data_dir = detect_asdf_data_dir();

    if let Ok(path) = which("asdf") {
        let version = get_asdf_version(&path).await;
        let nodejs_plugin_installed = has_nodejs_plugin(&path, asdf_data_dir.as_deref()).await;
        return build_detection(
            Some(path),
            version,
            true,
            asdf_data_dir,
            nodejs_plugin_installed,
        );
    }

    for path in get_common_asdf_paths() {
        if path.exists() {
            let version = get_asdf_version(&path).await;
            let nodejs_plugin_installed = has_nodejs_plugin(&path, asdf_data_dir.as_deref()).await;
            return build_detection(
                Some(path),
                version,
                false,
                asdf_data_dir,
                nodejs_plugin_installed,
            );
        }
    }

    build_detection(None, None, false, asdf_data_dir, false)
}

fn build_detection(
    path: Option<PathBuf>,
    version: Option<String>,
    in_path: bool,
    asdf_data_dir: Option<PathBuf>,
    nodejs_plugin_installed: bool,
) -> AsdfDetection {
    AsdfDetection {
        found: path.is_some() && nodejs_plugin_installed,
        path,
        version,
        in_path,
        asdf_data_dir,
    }
}

pub(crate) fn detect_asdf_data_dir() -> Option<PathBuf> {
    let env_dir = std::env::var("ASDF_DATA_DIR").ok().map(PathBuf::from);
    select_asdf_data_dir(env_dir, get_asdf_data_dir_candidates())
}

fn select_asdf_data_dir(env_dir: Option<PathBuf>, candidates: Vec<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = env_dir.filter(|path| path.exists()) {
        return Some(path);
    }

    candidates
        .iter()
        .find(|candidate| {
            candidate.exists()
                && candidate.join("plugins").exists()
                && candidate.join("installs").exists()
        })
        .cloned()
        .or_else(|| candidates.into_iter().find(|candidate| candidate.exists()))
}

fn get_asdf_data_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".asdf"));
    }

    if let Ok(custom) = std::env::var("ASDF_DATA_DIR") {
        candidates.push(PathBuf::from(custom));
    }

    candidates
}

fn get_common_asdf_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".asdf").join("bin").join("asdf"));
        paths.push(home.join(".local").join("bin").join("asdf"));
        paths.push(home.join("go").join("bin").join("asdf"));
    }

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/opt/homebrew/bin/asdf"));
    }

    #[cfg(unix)]
    {
        paths.push(PathBuf::from("/usr/local/bin/asdf"));
        paths.push(PathBuf::from("/usr/bin/asdf"));
    }

    paths
}

async fn get_asdf_version(path: &Path) -> Option<String> {
    let output = Command::new(path)
        .arg("--version")
        .hide_window()
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    normalize_asdf_version(&String::from_utf8_lossy(&output.stdout))
}

fn normalize_asdf_version(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_prefix = trimmed.strip_prefix("asdf ").unwrap_or(trimmed).trim();
    let token = without_prefix
        .strip_prefix('v')
        .unwrap_or(without_prefix)
        .trim();

    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

async fn has_nodejs_plugin(asdf_path: &Path, asdf_data_dir: Option<&Path>) -> bool {
    let mut cmd = Command::new(asdf_path);
    cmd.args(["plugin", "list"]);
    if let Some(dir) = asdf_data_dir {
        cmd.env("ASDF_DATA_DIR", dir);
    }
    cmd.hide_window();

    let Ok(output) = cmd.output().await else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .any(|line| line == "nodejs")
}

#[cfg(unix)]
struct ReleaseAssetInfo {
    download_url: String,
}

#[cfg(unix)]
pub(crate) async fn install_asdf(
    client: &reqwest::Client,
) -> Result<(), versi_backend::BackendError> {
    let release = fetch_latest_release(client).await?;
    let asset = select_release_asset(&release)?;
    let archive_bytes = download_release_asset(client, &asset.download_url).await?;

    let temp_dir = temp_install_dir();
    let install_result = async {
        let source_binary = extract_asdf_binary(&archive_bytes, &temp_dir).await?;
        let (target_binary, data_dir) = install_asdf_binary(&source_binary).await?;
        ensure_nodejs_plugin(&target_binary, &data_dir).await
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    install_result
}

#[cfg(windows)]
pub(crate) async fn install_asdf(
    _client: &reqwest::Client,
) -> Result<(), versi_backend::BackendError> {
    let mut failures = Vec::new();
    for attempt in asdf_windows_install_attempts() {
        match run_windows_installer_attempt(attempt).await {
            Ok(()) => return Ok(()),
            Err(error) => failures.push(error.to_string()),
        }
    }

    Err(versi_backend::BackendError::install_failed(
        "run installer command",
        format!("all asdf install attempts failed: {}", failures.join("; ")),
    ))
}

#[cfg(not(any(unix, windows)))]
pub(crate) async fn install_asdf(
    _client: &reqwest::Client,
) -> Result<(), versi_backend::BackendError> {
    Err(versi_backend::BackendError::install_failed(
        "unsupported platform flow",
        "Automatic asdf installation is unsupported on this platform.",
    ))
}

#[cfg(unix)]
async fn fetch_latest_release(
    client: &reqwest::Client,
) -> Result<GitHubRelease, versi_backend::BackendError> {
    let release_response = client
        .get(ASDF_RELEASES_API)
        .header("User-Agent", "versi")
        .send()
        .await
        .map_err(|error| {
            versi_backend::BackendError::install_failed(
                "fetch asdf release",
                format!("failed to fetch release metadata: {error}"),
            )
        })?;

    if !release_response.status().is_success() {
        return Err(versi_backend::BackendError::install_failed(
            "fetch asdf release",
            format!("release API returned status {}", release_response.status()),
        ));
    }

    release_response.json().await.map_err(|error| {
        versi_backend::BackendError::install_failed(
            "parse asdf release",
            format!("failed to parse release metadata: {error}"),
        )
    })
}

#[cfg(unix)]
fn select_release_asset(
    release: &GitHubRelease,
) -> Result<ReleaseAssetInfo, versi_backend::BackendError> {
    let release_version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();
    let asset_name = expected_archive_name(&release_version)?;

    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| {
            versi_backend::BackendError::install_failed(
                "locate asdf asset",
                format!("no matching release asset found: {asset_name}"),
            )
        })?;

    Ok(ReleaseAssetInfo {
        download_url: asset.browser_download_url.clone(),
    })
}

#[cfg(unix)]
async fn download_release_asset(
    client: &reqwest::Client,
    download_url: &str,
) -> Result<Vec<u8>, versi_backend::BackendError> {
    let response = client
        .get(download_url)
        .header("User-Agent", "versi")
        .send()
        .await
        .map_err(|error| {
            versi_backend::BackendError::install_failed(
                "download asdf asset",
                format!("failed to download archive: {error}"),
            )
        })?;

    if !response.status().is_success() {
        return Err(versi_backend::BackendError::install_failed(
            "download asdf asset",
            format!("asset download returned status {}", response.status()),
        ));
    }

    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| {
            versi_backend::BackendError::install_failed(
                "download asdf asset",
                format!("failed to read archive body: {error}"),
            )
        })
}

#[cfg(unix)]
async fn extract_asdf_binary(
    archive_bytes: &[u8],
    temp_dir: &Path,
) -> Result<PathBuf, versi_backend::BackendError> {
    tokio::fs::create_dir_all(temp_dir).await?;
    let archive_path = temp_dir.join("asdf.tar.gz");
    tokio::fs::write(&archive_path, archive_bytes).await?;

    let extract_dir = temp_dir.join("extract");
    tokio::fs::create_dir_all(&extract_dir).await?;

    let extract_status = Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&extract_dir)
        .hide_window()
        .status()
        .await
        .map_err(versi_backend::BackendError::from)?;

    if !extract_status.success() {
        return Err(versi_backend::BackendError::install_failed(
            "extract asdf archive",
            "failed to extract asdf archive with tar",
        ));
    }

    let source_binary = extract_dir.join("asdf");
    if source_binary.exists() {
        Ok(source_binary)
    } else {
        Err(versi_backend::BackendError::install_failed(
            "extract asdf archive",
            "asdf binary not found in extracted archive",
        ))
    }
}

#[cfg(unix)]
async fn install_asdf_binary(
    source_binary: &Path,
) -> Result<(PathBuf, PathBuf), versi_backend::BackendError> {
    let home = dirs::home_dir().ok_or_else(|| {
        versi_backend::BackendError::install_failed(
            "resolve home directory",
            "home directory unavailable",
        )
    })?;

    let data_dir = detect_asdf_data_dir().unwrap_or_else(|| home.join(".asdf"));
    let target_dir = data_dir.join("bin");
    tokio::fs::create_dir_all(&target_dir).await?;

    let target_binary = target_dir.join("asdf");
    tokio::fs::copy(source_binary, &target_binary).await?;
    std::fs::set_permissions(&target_binary, std::fs::Permissions::from_mode(0o755)).map_err(
        |error| {
            versi_backend::BackendError::install_failed(
                "set binary permissions",
                format!("failed to set executable permissions: {error}"),
            )
        },
    )?;

    Ok((target_binary, data_dir))
}

#[cfg(unix)]
fn expected_archive_name(release_version: &str) -> Result<String, versi_backend::BackendError> {
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return Err(versi_backend::BackendError::install_failed(
            "resolve platform",
            "automatic asdf installation is only supported on macOS/Linux",
        ));
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "386"
    } else {
        return Err(versi_backend::BackendError::install_failed(
            "resolve architecture",
            "automatic asdf installation is unsupported on this CPU architecture",
        ));
    };

    Ok(format!("asdf-v{release_version}-{os}-{arch}.tar.gz"))
}

#[cfg(unix)]
fn temp_install_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("versi-asdf-install-{}-{nonce}", std::process::id()))
}

#[cfg(unix)]
async fn ensure_nodejs_plugin(
    asdf_path: &Path,
    asdf_data_dir: &Path,
) -> Result<(), versi_backend::BackendError> {
    if has_nodejs_plugin(asdf_path, Some(asdf_data_dir)).await {
        return Ok(());
    }

    let output = Command::new(asdf_path)
        .args(["plugin", "add", "nodejs", ASDF_NODEJS_PLUGIN_URL])
        .env("ASDF_DATA_DIR", asdf_data_dir)
        .hide_window()
        .output()
        .await?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(versi_backend::BackendError::install_failed(
            "install nodejs plugin",
            if stderr.is_empty() {
                "failed to add nodejs plugin".to_string()
            } else {
                stderr
            },
        ))
    }
}

#[cfg(windows)]
async fn run_windows_installer_attempt(
    attempt: &InstallerAttempt,
) -> Result<(), versi_backend::BackendError> {
    let status = Command::new(attempt.program)
        .args(attempt.args)
        .hide_window()
        .status()
        .await
        .map_err(|error| {
            versi_backend::BackendError::install_failed(
                "run installer command",
                format!("{} failed to start: {error}", attempt.label),
            )
        })?;

    if status.success() {
        Ok(())
    } else {
        let code = status
            .code()
            .map_or_else(|| "unknown".to_string(), |code| code.to_string());
        Err(versi_backend::BackendError::install_failed(
            "run installer command",
            format!("{} exited with status {code}", attempt.label),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        asdf_windows_install_attempts, build_detection, get_common_asdf_paths,
        normalize_asdf_version, select_asdf_data_dir,
    };

    fn temp_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "versi-asdf-detection-test-{}-{nonce}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn select_asdf_data_dir_prefers_existing_env_dir() {
        let env_dir = temp_path("env");
        let candidate = temp_path("candidate");
        std::fs::create_dir_all(&env_dir).expect("create env dir");
        std::fs::create_dir_all(&candidate).expect("create candidate dir");

        let selected = select_asdf_data_dir(Some(env_dir.clone()), vec![candidate.clone()]);

        assert_eq!(selected, Some(env_dir.clone()));

        let _ = std::fs::remove_dir_all(candidate);
        let _ = std::fs::remove_dir_all(env_dir);
    }

    #[test]
    fn select_asdf_data_dir_prefers_candidate_with_plugins_and_installs() {
        let plain = temp_path("plain");
        let rich = temp_path("rich");

        std::fs::create_dir_all(&plain).expect("create plain dir");
        std::fs::create_dir_all(rich.join("plugins")).expect("create plugins dir");
        std::fs::create_dir_all(rich.join("installs")).expect("create installs dir");

        let selected = select_asdf_data_dir(None, vec![plain.clone(), rich.clone()]);

        assert_eq!(selected, Some(rich.clone()));

        let _ = std::fs::remove_dir_all(plain);
        let _ = std::fs::remove_dir_all(rich);
    }

    #[test]
    fn common_asdf_paths_are_unique() {
        let paths = get_common_asdf_paths();
        let unique: std::collections::HashSet<_> = paths.iter().collect();

        assert_eq!(paths.len(), unique.len());
    }

    #[test]
    fn normalize_asdf_version_strips_prefixes() {
        assert_eq!(
            normalize_asdf_version("asdf 0.18.0\n"),
            Some("0.18.0".into())
        );
        assert_eq!(normalize_asdf_version("v0.18.0"), Some("0.18.0".into()));
    }

    #[test]
    fn build_detection_requires_path_and_nodejs_plugin() {
        let base_path = Some(PathBuf::from("/usr/local/bin/asdf"));
        let detected = build_detection(base_path.clone(), None, true, None, true);
        let plugin_missing = build_detection(base_path, None, true, None, false);

        assert!(detected.found);
        assert!(!plugin_missing.found);
    }

    #[test]
    fn windows_install_attempts_are_ordered_by_preference() {
        let attempts = asdf_windows_install_attempts();

        assert_eq!(attempts.len(), 3);
        assert_eq!(attempts[0].program, "winget");
        assert_eq!(attempts[1].program, "scoop");
        assert_eq!(attempts[2].program, "choco");
    }
}
