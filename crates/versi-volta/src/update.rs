use versi_backend::{BackendError, BackendUpdate};
use versi_core::{GitHubRelease, backend_update_from_release};

const VOLTA_GITHUB_REPO: &str = "volta-cli/volta";

fn into_backend_update(info: versi_core::BackendUpdateInfo) -> BackendUpdate {
    BackendUpdate {
        current_version: info.current_version,
        latest_version: info.latest_version,
        release_url: info.release_url,
    }
}

pub async fn check_for_volta_update(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<Option<BackendUpdate>, BackendError> {
    let url = format!("https://api.github.com/repos/{VOLTA_GITHUB_REPO}/releases/latest");

    let response = client
        .get(&url)
        .header("User-Agent", "versi")
        .send()
        .await
        .map_err(|error| BackendError::network_request_from("volta update check", error))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|error| BackendError::network_parse_from("volta update check", error))?;

    Ok(backend_update_from_release(release, current_version).map(into_backend_update))
}

#[cfg(test)]
mod tests {
    use super::into_backend_update;
    use versi_core::{GitHubRelease, backend_update_from_release};

    fn release(tag_name: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag_name.to_string(),
            html_url: "https://github.com/volta-cli/volta/releases/tag/v1.0.0".to_string(),
            body: None,
            assets: Vec::new(),
        }
    }

    #[test]
    fn returns_update_when_release_is_newer() {
        let info = backend_update_from_release(release("v2.0.0"), "1.9.0")
            .expect("newer release should produce update metadata");
        let update = into_backend_update(info);

        assert_eq!(update.current_version, "1.9.0");
        assert_eq!(update.latest_version, "2.0.0");
    }

    #[test]
    fn returns_none_when_release_is_not_newer() {
        assert!(backend_update_from_release(release("v2.0.0"), "2.0.0").is_none());
        assert!(backend_update_from_release(release("v1.8.0"), "v1.9.0").is_none());
    }
}
