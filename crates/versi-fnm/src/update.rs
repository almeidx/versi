use versi_backend::BackendUpdate;
use versi_core::{GitHubRelease, is_newer_version};

const FNM_GITHUB_REPO: &str = "Schniz/fnm";

pub async fn check_for_fnm_update(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<Option<BackendUpdate>, String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        FNM_GITHUB_REPO
    );

    let response = client
        .get(&url)
        .header("User-Agent", "versi")
        .send()
        .await
        .map_err(|e| format!("Failed to check for fnm update: {}", e))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse fnm update response: {}", e))?;

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
