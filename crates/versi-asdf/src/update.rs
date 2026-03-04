use versi_backend::{BackendError, BackendUpdate};
use versi_core::{GitHubRelease, check_github_release, is_newer_version};

const ASDF_GITHUB_REPO: &str = "asdf-vm/asdf";

fn backend_update_from_release(
    release: GitHubRelease,
    current_version: &str,
) -> Option<BackendUpdate> {
    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let current = current_version
        .trim()
        .trim_start_matches("asdf ")
        .trim_start_matches('v');

    if is_newer_version(latest, current) {
        Some(BackendUpdate {
            current_version: current.to_string(),
            latest_version: latest.to_string(),
            release_url: release.html_url,
        })
    } else {
        None
    }
}

pub async fn check_for_asdf_update(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<Option<BackendUpdate>, BackendError> {
    let release = check_github_release(client, ASDF_GITHUB_REPO)
        .await
        .map_err(|error| BackendError::network_request_from("asdf update check", error))?;

    let Some(release) = release else {
        return Ok(None);
    };

    Ok(backend_update_from_release(release, current_version))
}

#[cfg(test)]
mod tests {
    use super::{GitHubRelease, backend_update_from_release};

    fn release(tag_name: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag_name.to_string(),
            html_url: "https://github.com/asdf-vm/asdf/releases/tag/v0.18.0".to_string(),
            body: None,
            assets: Vec::new(),
        }
    }

    #[test]
    fn returns_update_when_release_is_newer() {
        let update = backend_update_from_release(release("v0.18.0"), "asdf 0.17.0")
            .expect("newer release should produce update metadata");

        assert_eq!(update.current_version, "0.17.0");
        assert_eq!(update.latest_version, "0.18.0");
    }

    #[test]
    fn returns_none_when_release_is_not_newer() {
        assert!(backend_update_from_release(release("v0.18.0"), "0.18.0").is_none());
        assert!(backend_update_from_release(release("v0.16.0"), "0.18.0").is_none());
    }
}
