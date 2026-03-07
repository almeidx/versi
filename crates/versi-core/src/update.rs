use semver::Version;
use serde::Deserialize;
use thiserror::Error;

use crate::http::{USER_AGENT, response_snippet};

const GITHUB_REPO: &str = "almeidx/versi";

#[derive(Debug, Clone)]
pub struct AppUpdate {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub download_size: Option<u64>,
    pub download_sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub html_url: String,
    pub body: Option<String>,
    #[serde(default)]
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("failed to check for app update: {0}")]
    Request(#[source] reqwest::Error),
    #[error("app update check failed with HTTP {status}{body_snippet}")]
    HttpStatus {
        status: reqwest::StatusCode,
        body_snippet: String,
    },
    #[error("failed to parse app update response: {0}")]
    Parse(#[source] reqwest::Error),
}

fn asset_name(version: &str) -> Option<String> {
    let name = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        format!("versi-{version}-macos-arm64.zip")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        format!("versi-{version}-macos-x64.zip")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        format!("versi-{version}-linux-x64.zip")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        format!("versi-{version}-linux-arm64.zip")
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        format!("versi-{version}-windows-x64.msi")
    } else {
        return None;
    };
    Some(name)
}

/// Check GitHub releases for a newer Versi version.
///
/// # Errors
/// Returns an error when the update API request fails or the release response
/// cannot be parsed.
pub async fn check_for_update(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<Option<AppUpdate>, UpdateError> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");

    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(UpdateError::Request)?;

    if !response.status().is_success() {
        let status = response.status();
        let body_snippet = response
            .text()
            .await
            .ok()
            .map(|body| response_snippet(&body, 160))
            .unwrap_or_default();
        return Err(UpdateError::HttpStatus {
            status,
            body_snippet,
        });
    }

    let release: GitHubRelease = response.json().await.map_err(UpdateError::Parse)?;

    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let current = current_version.strip_prefix('v').unwrap_or(current_version);

    if is_newer_version(latest, current) {
        let (download_url, download_size, download_sha256) = asset_name(latest)
            .and_then(|expected| {
                release
                    .assets
                    .iter()
                    .find(|a| a.name == expected)
                    .and_then(|a| {
                        parse_sha256_digest(a.digest.as_deref()?).map(|digest| {
                            (
                                Some(a.browser_download_url.clone()),
                                Some(a.size),
                                Some(digest),
                            )
                        })
                    })
            })
            .unwrap_or((None, None, None));

        Ok(Some(AppUpdate {
            current_version: current.to_string(),
            latest_version: latest.to_string(),
            release_url: release.html_url,
            release_notes: release.body,
            download_url,
            download_size,
            download_sha256,
        }))
    } else {
        Ok(None)
    }
}

/// Fetch the latest GitHub release for a backend's repository and return
/// update metadata when a newer version is available.
///
/// # Errors
/// Returns an error when the HTTP request itself fails or the successful
/// response body cannot be deserialized.
pub async fn check_github_backend_update(
    client: &reqwest::Client,
    repo: &str,
    current_version: &str,
) -> Result<Option<BackendUpdateInfo>, UpdateError> {
    let release = check_github_release(client, repo).await?;

    let Some(release) = release else {
        return Ok(None);
    };

    Ok(backend_update_from_release(release, current_version))
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

fn parse_semver(version: &str) -> Option<Version> {
    if let Ok(parsed) = Version::parse(version) {
        return Some(parsed);
    }

    let (core, suffix) = split_semver_core_and_suffix(version);
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().and_then(|part| part.parse::<u64>().ok());
    let patch = parts.next().and_then(|part| part.parse::<u64>().ok());

    if parts.next().is_some() {
        return None;
    }

    let normalized = match (minor, patch) {
        (None, None) => format!("{major}.0.0{suffix}"),
        (Some(minor), None) => format!("{major}.{minor}.0{suffix}"),
        (Some(minor), Some(patch)) => format!("{major}.{minor}.{patch}{suffix}"),
        (None, Some(_)) => return None,
    };

    Version::parse(&normalized).ok()
}

fn split_semver_core_and_suffix(version: &str) -> (&str, &str) {
    let suffix_idx = version.find(['-', '+']).unwrap_or(version.len());
    (&version[..suffix_idx], &version[suffix_idx..])
}

#[must_use]
pub fn parse_sha256_digest(digest: &str) -> Option<String> {
    let (algorithm, hash) = digest.split_once(':')?;
    if !algorithm.eq_ignore_ascii_case("sha256") {
        return None;
    }
    if hash.len() != 64 || !hash.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    Some(hash.to_ascii_lowercase())
}

/// Fetch the latest GitHub release for a repository.
///
/// Returns `Ok(None)` when the API responds with a non-success status (e.g.
/// rate-limited or repo not found), letting callers treat that as "no update
/// available" rather than a hard failure.
///
/// # Errors
/// Returns an error when the HTTP request itself fails or the successful
/// response body cannot be deserialized.
async fn check_github_release(
    client: &reqwest::Client,
    repo: &str,
) -> Result<Option<GitHubRelease>, UpdateError> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");

    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(UpdateError::Request)?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let release: GitHubRelease = response.json().await.map_err(UpdateError::Parse)?;

    Ok(Some(release))
}

#[derive(Debug, Clone)]
pub struct BackendUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
}

#[must_use]
pub fn backend_update_from_release(
    release: GitHubRelease,
    current_version: &str,
) -> Option<BackendUpdateInfo> {
    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let current = current_version.strip_prefix('v').unwrap_or(current_version);

    if is_newer_version(latest, current) {
        Some(BackendUpdateInfo {
            current_version: current.to_string(),
            latest_version: latest.to_string(),
            release_url: release.html_url,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison() {
        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("1.1.0", "1.0.0"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(is_newer_version("1.2", "1.1.9"));
        assert!(is_newer_version("1", "0.99.0"));
        assert!(is_newer_version("1.0.0", "1.0.0-beta.2"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("1.2", "1.2.0"));
        assert!(!is_newer_version("1.0.0-beta.2", "1.0.0-beta.10"));
        assert!(!is_newer_version("1.0.0", "1.0.1"));
        assert!(!is_newer_version("0.9.0", "1.0.0"));
        assert!(!is_newer_version("nightly", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "nightly"));
        assert!(!is_newer_version("rc-test", "abc"));
    }

    #[test]
    fn parse_sha256_digest_accepts_valid_sha256() {
        let parsed = parse_sha256_digest(
            "sha256:50639d63848d275a7efcd04478de62ca0df8f35dfd75be490e4fcae667ecd436",
        );
        assert_eq!(
            parsed.as_deref(),
            Some("50639d63848d275a7efcd04478de62ca0df8f35dfd75be490e4fcae667ecd436")
        );
    }

    #[test]
    fn parse_sha256_digest_rejects_invalid_values() {
        assert!(parse_sha256_digest("sha1:abc").is_none());
        assert!(parse_sha256_digest("sha256:not-hex").is_none());
        assert!(parse_sha256_digest("sha256:abcd").is_none());
    }

    #[test]
    fn parse_sha256_digest_rejects_missing_colon() {
        assert!(parse_sha256_digest("sha256").is_none());
        assert!(parse_sha256_digest("").is_none());
    }

    #[test]
    fn parse_sha256_digest_lowercases_output() {
        let parsed = parse_sha256_digest(
            "SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        );
        assert_eq!(
            parsed.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }

    #[test]
    fn split_semver_core_and_suffix_splits_on_hyphen() {
        assert_eq!(
            split_semver_core_and_suffix("1.0.0-beta.1"),
            ("1.0.0", "-beta.1")
        );
    }

    #[test]
    fn split_semver_core_and_suffix_splits_on_plus() {
        assert_eq!(
            split_semver_core_and_suffix("1.0.0+build.42"),
            ("1.0.0", "+build.42")
        );
    }

    #[test]
    fn split_semver_core_and_suffix_returns_full_string_when_no_suffix() {
        assert_eq!(split_semver_core_and_suffix("2.3.4"), ("2.3.4", ""));
    }

    #[test]
    fn parse_semver_handles_standard_semver() {
        let v = parse_semver("1.2.3").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (1, 2, 3));
    }

    #[test]
    fn parse_semver_fills_missing_minor_and_patch() {
        let v = parse_semver("5").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (5, 0, 0));

        let v = parse_semver("3.7").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (3, 7, 0));
    }

    #[test]
    fn parse_semver_preserves_prerelease_suffix() {
        let v = parse_semver("1.0.0-alpha.1").unwrap();
        assert!(!v.pre.is_empty());

        let v = parse_semver("2-rc.1").unwrap();
        assert!(!v.pre.is_empty());
    }

    #[test]
    fn parse_semver_rejects_too_many_parts() {
        assert!(parse_semver("1.2.3.4").is_none());
    }

    #[test]
    fn parse_semver_rejects_non_numeric() {
        assert!(parse_semver("abc").is_none());
        assert!(parse_semver("").is_none());
    }

    #[test]
    fn backend_update_from_release_returns_some_when_newer() {
        let release = GitHubRelease {
            tag_name: "v2.0.0".to_string(),
            html_url: "https://github.com/example/releases/v2.0.0".to_string(),
            body: Some("release notes".to_string()),
            assets: vec![],
        };

        let result = backend_update_from_release(release, "1.5.0");
        let info = result.expect("should detect newer version");
        assert_eq!(info.current_version, "1.5.0");
        assert_eq!(info.latest_version, "2.0.0");
    }

    #[test]
    fn backend_update_from_release_returns_none_when_same() {
        let release = GitHubRelease {
            tag_name: "v1.5.0".to_string(),
            html_url: "https://github.com/example/releases/v1.5.0".to_string(),
            body: None,
            assets: vec![],
        };

        assert!(backend_update_from_release(release, "1.5.0").is_none());
    }

    #[test]
    fn backend_update_from_release_strips_v_prefix_from_both_sides() {
        let release = GitHubRelease {
            tag_name: "v3.0.0".to_string(),
            html_url: "https://example.com".to_string(),
            body: None,
            assets: vec![],
        };

        let info = backend_update_from_release(release, "v2.0.0").unwrap();
        assert_eq!(info.current_version, "2.0.0");
        assert_eq!(info.latest_version, "3.0.0");
    }
}
