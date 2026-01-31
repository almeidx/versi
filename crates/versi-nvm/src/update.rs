use serde::Deserialize;
use versi_backend::BackendUpdate;

use crate::detection::NvmVariant;

const NVM_UNIX_REPO: &str = "nvm-sh/nvm";
const NVM_WINDOWS_REPO: &str = "coreybutler/nvm-windows";

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

pub async fn check_for_nvm_update(
    client: &reqwest::Client,
    current_version: &str,
    variant: &NvmVariant,
) -> Result<Option<BackendUpdate>, String> {
    let repo = match variant {
        NvmVariant::Unix | NvmVariant::NotFound => NVM_UNIX_REPO,
        NvmVariant::Windows => NVM_WINDOWS_REPO,
    };

    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let response = client
        .get(&url)
        .header("User-Agent", "versi")
        .send()
        .await
        .map_err(|e| format!("Failed to check for nvm update: {}", e))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse nvm update response: {}", e))?;

    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let current = current_version.strip_prefix('v').unwrap_or(current_version);

    if is_newer_version(latest, current) {
        Ok(Some(BackendUpdate {
            current_version: current.to_string(),
            latest_version: latest.to_string(),
            release_url: release.html_url,
        }))
    } else {
        Ok(None)
    }
}

fn is_newer_version(latest: &str, current: &str) -> bool {
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
        (Some(l), Some(c)) => l > c,
        _ => latest != current,
    }
}
