use serde::Deserialize;

const GITHUB_REPO: &str = "almeidx/versi";

#[derive(Debug, Clone)]
pub struct AppUpdate {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub download_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub html_url: String,
    pub body: Option<String>,
    #[serde(default)]
    pub assets: Vec<GitHubAsset>,
}

pub fn asset_name(version: &str) -> Option<String> {
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

pub async fn check_for_update(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<Option<AppUpdate>, String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = client
        .get(&url)
        .header("User-Agent", "versi")
        .send()
        .await
        .map_err(|e| format!("Failed to check for app update: {}", e))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse app update response: {}", e))?;

    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let current = current_version.strip_prefix('v').unwrap_or(current_version);

    if is_newer_version(latest, current) {
        let (download_url, download_size) = asset_name(latest)
            .and_then(|expected| {
                release
                    .assets
                    .iter()
                    .find(|a| a.name == expected)
                    .map(|a| (Some(a.browser_download_url.clone()), Some(a.size)))
            })
            .unwrap_or((None, None));

        Ok(Some(AppUpdate {
            current_version: current.to_string(),
            latest_version: latest.to_string(),
            release_url: release.html_url,
            release_notes: release.body,
            download_url,
            download_size,
        }))
    } else {
        Ok(None)
    }
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() >= 3 {
            Some((
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].parse().ok()?,
            ))
        } else if parts.len() == 2 {
            Some((parts[0].parse().ok()?, parts[1].parse().ok()?, 0))
        } else if parts.len() == 1 {
            Some((parts[0].parse().ok()?, 0, 0))
        } else {
            None
        }
    };

    match (parse_version(latest), parse_version(current)) {
        (Some((l_major, l_minor, l_patch)), Some((c_major, c_minor, c_patch))) => {
            (l_major, l_minor, l_patch) > (c_major, c_minor, c_patch)
        }
        _ => latest != current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("1.1.0", "1.0.0"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.1"));
        assert!(!is_newer_version("0.9.0", "1.0.0"));
    }
}
