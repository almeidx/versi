use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

const INSTALL_SCRIPT_TIMEOUT: Duration = Duration::from_secs(30);
const INSTALL_SCRIPT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const INSTALL_SCRIPT_RETRY_DELAYS_SECS: [u64; 3] = [0, 2, 5];

#[derive(Debug, thiserror::Error)]
pub enum InstallScriptError {
    #[error("failed to build installer download client: {0}")]
    ClientBuild(reqwest::Error),
    #[error("failed to download installer script from {url}: {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("installer script download failed with HTTP {status} for {url}")]
    Status {
        url: String,
        status: reqwest::StatusCode,
    },
    #[error("installer script checksum mismatch: expected {expected_sha256}, got {actual_sha256}")]
    ChecksumMismatch {
        expected_sha256: String,
        actual_sha256: String,
    },
    #[error("failed to write installer script to {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to set permissions on installer script {path}: {source}")]
    SetPermissions {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[must_use]
pub fn temp_script_path(prefix: &str, ext: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("{prefix}-{}-{nonce}.{ext}", std::process::id()))
}

/// Download an installer script with timeout/retry policy and SHA-256
/// verification.
///
/// # Errors
/// Returns an error if the HTTP request fails, the server responds with a
/// non-success status, checksum verification fails, or writing the script to
/// disk fails.
pub async fn download_install_script_verified(
    url: &str,
    expected_sha256: &str,
    path: &Path,
) -> Result<(), InstallScriptError> {
    let client = build_download_client()?;

    let script = download_with_retries(&client, url).await?;
    verify_checksum(&script, expected_sha256)?;
    write_script(path, &script).await
}

/// Download an installer script with timeout/retry policy and no checksum
/// verification.
///
/// # Errors
/// Returns an error if the HTTP request fails, the server responds with a
/// non-success status, or writing the script to disk fails.
pub async fn download_install_script_unverified(
    url: &str,
    path: &Path,
) -> Result<(), InstallScriptError> {
    let client = build_download_client()?;
    let script = download_with_retries(&client, url).await?;
    write_script(path, &script).await
}

/// Download an installer script with timeout/retry policy, no checksum
/// verification, and (on Unix) set the file executable.
///
/// # Errors
/// Returns an error if the HTTP request fails, the server responds with a
/// non-success status, writing the script to disk fails, or setting
/// permissions fails.
pub async fn download_install_script(url: &str, path: &Path) -> Result<(), InstallScriptError> {
    download_install_script_unverified(url, path).await?;
    set_executable(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), InstallScriptError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(|source| {
        InstallScriptError::SetPermissions {
            path: path.display().to_string(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), InstallScriptError> {
    Ok(())
}

fn build_download_client() -> Result<reqwest::Client, InstallScriptError> {
    reqwest::Client::builder()
        .timeout(INSTALL_SCRIPT_TIMEOUT)
        .connect_timeout(INSTALL_SCRIPT_CONNECT_TIMEOUT)
        .user_agent(format!(
            "versi/{}/installer-script",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(InstallScriptError::ClientBuild)
}

async fn write_script(path: &Path, script: &[u8]) -> Result<(), InstallScriptError> {
    tokio::fs::write(path, &script)
        .await
        .map_err(|source| InstallScriptError::Write {
            path: path.display().to_string(),
            source,
        })?;
    Ok(())
}

async fn download_with_retries(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<u8>, InstallScriptError> {
    let mut last_error = None;

    for delay_secs in INSTALL_SCRIPT_RETRY_DELAYS_SECS {
        if delay_secs > 0 {
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        }

        match download_once(client, url).await {
            Ok(bytes) => return Ok(bytes),
            Err(error) => {
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| InstallScriptError::Status {
        url: url.to_string(),
        status: reqwest::StatusCode::REQUEST_TIMEOUT,
    }))
}

async fn download_once(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, InstallScriptError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|source| InstallScriptError::Request {
            url: url.to_string(),
            source,
        })?;

    if !response.status().is_success() {
        return Err(InstallScriptError::Status {
            url: url.to_string(),
            status: response.status(),
        });
    }

    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|source| InstallScriptError::Request {
            url: url.to_string(),
            source,
        })
}

fn verify_checksum(script: &[u8], expected_sha256: &str) -> Result<(), InstallScriptError> {
    let actual_sha256 = format!("{:x}", Sha256::digest(script));
    if actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        return Ok(());
    }

    Err(InstallScriptError::ChecksumMismatch {
        expected_sha256: expected_sha256.to_ascii_lowercase(),
        actual_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::verify_checksum;

    #[test]
    fn verify_checksum_accepts_matching_digest() {
        let script = b"abc";
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_checksum(script, expected).is_ok());
    }

    #[test]
    fn verify_checksum_rejects_mismatch() {
        let script = b"abc";
        let expected = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(verify_checksum(script, expected).is_err());
    }
}
