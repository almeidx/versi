use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine;
use serde::Deserialize;

const INSTALL_SCRIPT_TIMEOUT: Duration = Duration::from_secs(30);
const INSTALL_SCRIPT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

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
    #[error("failed to create temporary script file: {0}")]
    TempFile(std::io::Error),
    #[error("failed to fetch latest release tag for {repo}: {reason}")]
    LatestTag { repo: String, reason: String },
    #[error("GitHub content response for {repo}/{path} missing or undecodable content")]
    ContentDecode { repo: String, path: String },
}

/// Create a temporary script file with a unique, unpredictable name.
///
/// Uses `tempfile::Builder` with `O_EXCL` semantics to prevent symlink attacks.
///
/// # Errors
/// Returns an error if the temp file cannot be created.
pub fn temp_script_path(prefix: &str, ext: &str) -> Result<PathBuf, InstallScriptError> {
    let suffix = format!(".{ext}");
    let named = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(&suffix)
        .tempfile()
        .map_err(InstallScriptError::TempFile)?;
    let (_, path) = named
        .keep()
        .map_err(|e| InstallScriptError::TempFile(e.error))?;
    Ok(path)
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
        .user_agent(crate::http::USER_AGENT)
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

/// Download an install script from a GitHub repository using the latest
/// release tag, verified through the GitHub Contents API.
///
/// Instead of downloading from a raw URL (which offers no integrity metadata),
/// this fetches the file content via the GitHub Contents API at the latest
/// release ref. The API returns base64-encoded content that is decoded and
/// written to `dest`.
///
/// # Errors
/// Returns an error if the latest release tag cannot be fetched, the file
/// content cannot be retrieved or decoded, or writing to disk fails.
pub async fn download_github_install_script(
    owner: &str,
    repo: &str,
    script_path: &str,
    dest: &Path,
) -> Result<(), InstallScriptError> {
    let client = build_download_client()?;

    let tag = fetch_latest_github_tag(&client, owner, repo).await?;
    log::info!("Resolved latest release for {owner}/{repo}: {tag}");

    let script = fetch_github_file_content(&client, owner, repo, script_path, &tag).await?;
    log::info!(
        "Downloaded {script_path} from {owner}/{repo}@{tag} ({} bytes)",
        script.len()
    );

    write_script(dest, &script).await?;
    set_executable(dest)?;
    Ok(())
}

async fn fetch_latest_github_tag(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
) -> Result<String, InstallScriptError> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");

    let response = client
        .get(&url)
        .header("User-Agent", crate::http::USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|source| InstallScriptError::Request {
            url: url.clone(),
            source,
        })?;

    if !response.status().is_success() {
        return Err(InstallScriptError::LatestTag {
            repo: format!("{owner}/{repo}"),
            reason: format!("HTTP {}", response.status()),
        });
    }

    let release: GitHubLatestRelease = response
        .json()
        .await
        .map_err(|source| InstallScriptError::Request { url, source })?;

    Ok(release.tag_name)
}

#[derive(Deserialize)]
struct GitHubLatestRelease {
    tag_name: String,
}

#[derive(Deserialize)]
struct GitHubFileContent {
    content: Option<String>,
}

async fn fetch_github_file_content(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    path: &str,
    git_ref: &str,
) -> Result<Vec<u8>, InstallScriptError> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={git_ref}");

    let response = client
        .get(&url)
        .header("User-Agent", crate::http::USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|source| InstallScriptError::Request {
            url: url.clone(),
            source,
        })?;

    if !response.status().is_success() {
        return Err(InstallScriptError::Status {
            url,
            status: response.status(),
        });
    }

    let file: GitHubFileContent =
        response
            .json()
            .await
            .map_err(|source| InstallScriptError::Request {
                url: url.clone(),
                source,
            })?;

    let encoded = file
        .content
        .ok_or_else(|| InstallScriptError::ContentDecode {
            repo: format!("{owner}/{repo}"),
            path: path.to_string(),
        })?;

    // GitHub returns base64 with embedded newlines
    let cleaned: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();

    base64::engine::general_purpose::STANDARD
        .decode(&cleaned)
        .map_err(|_| InstallScriptError::ContentDecode {
            repo: format!("{owner}/{repo}"),
            path: path.to_string(),
        })
}
