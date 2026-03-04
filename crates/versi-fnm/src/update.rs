use versi_backend::{BackendError, BackendUpdate};
use versi_core::{backend_update_from_release, check_github_release};

const FNM_GITHUB_REPO: &str = "Schniz/fnm";

fn into_backend_update(info: versi_core::BackendUpdateInfo) -> BackendUpdate {
    BackendUpdate {
        current_version: info.current_version,
        latest_version: info.latest_version,
        release_url: info.release_url,
    }
}

pub async fn check_for_fnm_update(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<Option<BackendUpdate>, BackendError> {
    let release = check_github_release(client, FNM_GITHUB_REPO)
        .await
        .map_err(|error| BackendError::network_request_from("fnm update check", error))?;

    let Some(release) = release else {
        return Ok(None);
    };

    Ok(backend_update_from_release(release, current_version).map(into_backend_update))
}

#[cfg(test)]
mod tests {
    use super::into_backend_update;
    use versi_core::{GitHubRelease, backend_update_from_release};

    fn release(tag_name: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag_name.to_string(),
            html_url: "https://github.com/Schniz/fnm/releases/tag/v1.0.0".to_string(),
            body: None,
            assets: Vec::new(),
        }
    }

    #[test]
    fn returns_update_when_release_is_newer() {
        let info = backend_update_from_release(release("v1.38.0"), "v1.37.1")
            .expect("newer release should produce update metadata");
        let update = into_backend_update(info);

        assert_eq!(update.current_version, "1.37.1");
        assert_eq!(update.latest_version, "1.38.0");
        assert_eq!(
            update.release_url,
            "https://github.com/Schniz/fnm/releases/tag/v1.0.0"
        );
    }

    #[test]
    fn returns_none_when_release_is_not_newer() {
        assert!(backend_update_from_release(release("v1.38.0"), "1.38.0").is_none());
        assert!(backend_update_from_release(release("v1.37.0"), "v1.38.0").is_none());
    }
}
