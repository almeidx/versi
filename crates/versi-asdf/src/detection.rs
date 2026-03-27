use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use tokio::process::Command;
use which::which;

use versi_backend::BackendDetection;
#[cfg(unix)]
use versi_core::GitHubRelease;
use versi_core::HideWindow;
#[cfg(any(test, windows))]
use versi_core::InstallerAttempt;

#[cfg(unix)]
const ASDF_RELEASES_API: &str = "https://api.github.com/repos/asdf-vm/asdf/releases/latest";
#[cfg(unix)]
const ASDF_NODEJS_PLUGIN_URL: &str = "https://github.com/asdf-vm/asdf-nodejs.git";

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

pub(crate) async fn detect_asdf() -> BackendDetection {
    let data_dir = detect_asdf_data_dir();

    if let Ok(path) = which("asdf") {
        let version = get_asdf_version(&path).await;
        let nodejs_plugin_installed = has_nodejs_plugin(&path, data_dir.as_deref()).await;
        return build_detection(Some(path), version, true, data_dir, nodejs_plugin_installed);
    }

    for path in get_common_asdf_paths() {
        if path.exists() {
            let version = get_asdf_version(&path).await;
            let nodejs_plugin_installed = has_nodejs_plugin(&path, data_dir.as_deref()).await;
            return build_detection(
                Some(path),
                version,
                false,
                data_dir,
                nodejs_plugin_installed,
            );
        }
    }

    build_detection(None, None, false, data_dir, false)
}

fn build_detection(
    path: Option<PathBuf>,
    version: Option<String>,
    in_path: bool,
    data_dir: Option<PathBuf>,
    nodejs_plugin_installed: bool,
) -> BackendDetection {
    BackendDetection {
        found: path.is_some() && nodejs_plugin_installed,
        path,
        version,
        in_path,
        data_dir,
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
    sha256: Option<String>,
}

#[cfg(unix)]
pub(crate) async fn install_asdf(
    client: &reqwest::Client,
) -> Result<(), versi_backend::BackendError> {
    let release = fetch_latest_release(client).await?;
    let asset = select_release_asset(&release)?;
    let archive_bytes =
        download_release_asset(client, &asset.download_url, asset.sha256.as_deref()).await?;

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
        match versi_core::run_installer_attempt(attempt).await {
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
        .header("User-Agent", versi_core::http::USER_AGENT)
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

    let sha256 = asset
        .digest
        .as_deref()
        .and_then(versi_core::parse_sha256_digest);

    Ok(ReleaseAssetInfo {
        download_url: asset.browser_download_url.clone(),
        sha256,
    })
}

#[cfg(unix)]
async fn download_release_asset(
    client: &reqwest::Client,
    download_url: &str,
    expected_sha256: Option<&str>,
) -> Result<Vec<u8>, versi_backend::BackendError> {
    let response = client
        .get(download_url)
        .header("User-Agent", versi_core::http::USER_AGENT)
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

    let bytes = response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| {
            versi_backend::BackendError::install_failed(
                "download asdf asset",
                format!("failed to read archive body: {error}"),
            )
        })?;

    verify_asset_checksum(&bytes, expected_sha256)?;

    Ok(bytes)
}

#[cfg(unix)]
fn verify_asset_checksum(
    bytes: &[u8],
    expected_sha256: Option<&str>,
) -> Result<(), versi_backend::BackendError> {
    let Some(expected) = expected_sha256 else {
        return Err(versi_backend::BackendError::install_failed(
            "verify asdf checksum",
            "release asset is missing SHA-256 digest; refusing to install unverified binary",
        ));
    };

    let actual = versi_core::sha256_hex(bytes);

    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(versi_backend::BackendError::install_failed(
            "verify asdf checksum",
            "SHA-256 checksum mismatch; refusing to install tampered binary",
        ))
    }
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

    let entries = list_asdf_archive_entries(&archive_path).await?;
    let archive_binary_path = select_asdf_archive_binary(&entries)?;
    let source_binary = extract_dir.join("asdf");

    let extract_output = Command::new("tar")
        .arg("-xOzf")
        .arg(&archive_path)
        .arg(archive_binary_path)
        .hide_window()
        .output()
        .await
        .map_err(versi_backend::BackendError::from)?;

    if !extract_output.status.success() {
        let stderr = String::from_utf8_lossy(&extract_output.stderr)
            .trim()
            .to_string();
        return Err(versi_backend::BackendError::install_failed(
            "extract asdf archive",
            if stderr.is_empty() {
                "failed to extract asdf binary from archive".to_string()
            } else {
                format!("failed to extract asdf binary from archive: {stderr}")
            },
        ));
    }

    tokio::fs::write(&source_binary, &extract_output.stdout).await?;

    Ok(source_binary)
}

#[cfg(unix)]
async fn list_asdf_archive_entries(
    archive_path: &Path,
) -> Result<Vec<PathBuf>, versi_backend::BackendError> {
    let output = Command::new("tar")
        .arg("-tzf")
        .arg(archive_path)
        .hide_window()
        .output()
        .await
        .map_err(versi_backend::BackendError::from)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(versi_backend::BackendError::install_failed(
            "extract asdf archive",
            if stderr.is_empty() {
                "failed to list asdf archive contents".to_string()
            } else {
                format!("failed to list asdf archive contents: {stderr}")
            },
        ));
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .map(|path| {
            validate_archive_path(&path)?;
            Ok(path)
        })
        .collect()
}

#[cfg(unix)]
fn select_asdf_archive_binary(entries: &[PathBuf]) -> Result<&Path, versi_backend::BackendError> {
    let candidates: Vec<_> = entries
        .iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|file_name| file_name == std::ffi::OsStr::new("asdf"))
        })
        .collect();

    if let Some(path) = candidates
        .iter()
        .find(|path| normalize_archive_entry(path) == Path::new("asdf"))
    {
        return Ok(path.as_path());
    }

    let [first] = candidates.as_slice() else {
        return if candidates.is_empty() {
            Err(versi_backend::BackendError::install_failed(
                "extract asdf archive",
                "asdf binary not found in archive",
            ))
        } else {
            Err(versi_backend::BackendError::install_failed(
                "extract asdf archive",
                "multiple asdf binary entries found in archive",
            ))
        };
    };

    Ok(first.as_path())
}

#[cfg(unix)]
fn validate_archive_path(path: &Path) -> Result<(), versi_backend::BackendError> {
    use std::path::Component;

    if path.as_os_str().is_empty() {
        return Err(versi_backend::BackendError::install_failed(
            "extract asdf archive",
            "archive entry has an empty path",
        ));
    }

    for component in path.components() {
        match component {
            Component::CurDir | Component::Normal(_) => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(versi_backend::BackendError::install_failed(
                    "extract asdf archive",
                    format!("archive entry contains an unsafe path: {}", path.display()),
                ));
            }
        }
    }

    Ok(())
}

#[cfg(unix)]
fn normalize_archive_entry(path: &Path) -> &Path {
    path.strip_prefix(".").unwrap_or(path)
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

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::path::Path;
    use std::path::PathBuf;
    #[cfg(unix)]
    use tokio::process::Command;

    use super::{
        asdf_windows_install_attempts, build_detection, get_common_asdf_paths,
        normalize_asdf_version, select_asdf_data_dir,
    };
    #[cfg(unix)]
    use super::{extract_asdf_binary, validate_archive_path};

    #[test]
    fn select_asdf_data_dir_prefers_existing_env_dir() {
        let env_dir = tempfile::tempdir().expect("create env dir");
        let candidate = tempfile::tempdir().expect("create candidate dir");

        let selected = select_asdf_data_dir(
            Some(env_dir.path().to_path_buf()),
            vec![candidate.path().to_path_buf()],
        );

        assert_eq!(selected, Some(env_dir.path().to_path_buf()));
    }

    #[test]
    fn select_asdf_data_dir_prefers_candidate_with_plugins_and_installs() {
        let plain = tempfile::tempdir().expect("create plain dir");
        let rich = tempfile::tempdir().expect("create rich dir");

        std::fs::create_dir_all(rich.path().join("plugins")).expect("create plugins dir");
        std::fs::create_dir_all(rich.path().join("installs")).expect("create installs dir");

        let selected = select_asdf_data_dir(
            None,
            vec![plain.path().to_path_buf(), rich.path().to_path_buf()],
        );

        assert_eq!(selected, Some(rich.path().to_path_buf()));
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

    #[cfg(unix)]
    #[test]
    fn validate_archive_path_accepts_relative_paths() {
        assert!(validate_archive_path(Path::new("asdf")).is_ok());
        assert!(validate_archive_path(Path::new("./bin/asdf")).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn validate_archive_path_rejects_parent_dir_components() {
        let error = validate_archive_path(Path::new("../asdf")).expect_err("reject traversal");
        assert!(error.to_string().contains("unsafe path"));
    }

    #[cfg(unix)]
    #[test]
    fn validate_archive_path_rejects_absolute_paths() {
        let error = validate_archive_path(Path::new("/tmp/asdf")).expect_err("reject absolute");
        assert!(error.to_string().contains("unsafe path"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn extract_asdf_binary_extracts_safe_archive() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let archive_bytes = build_test_archive(temp_dir.path()).await;

        let extracted = extract_asdf_binary(&archive_bytes, temp_dir.path())
            .await
            .expect("extract safe archive");

        assert_eq!(extracted, temp_dir.path().join("extract").join("asdf"));
        assert!(extracted.exists());
    }

    #[cfg(unix)]
    async fn build_test_archive(base_dir: &Path) -> Vec<u8> {
        let source_dir = base_dir.join("archive-src");
        std::fs::create_dir_all(&source_dir).expect("create archive source dir");
        std::fs::write(source_dir.join("asdf"), b"asdf").expect("write source binary");

        let archive_path = base_dir.join("asdf.tar.gz");
        let output = Command::new("tar")
            .arg("-czf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&source_dir)
            .arg("asdf")
            .output()
            .await
            .expect("run tar to create archive");

        assert!(
            output.status.success(),
            "tar failed to create archive: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        tokio::fs::read(&archive_path)
            .await
            .expect("read generated archive")
    }
}
