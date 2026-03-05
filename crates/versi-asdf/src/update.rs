use versi_backend::{BackendError, BackendUpdate};
use versi_core::check_github_backend_update;

const ASDF_GITHUB_REPO: &str = "asdf-vm/asdf";

fn normalize_asdf_version(version: &str) -> &str {
    version.trim().trim_start_matches("asdf ")
}

pub(crate) async fn check_for_asdf_update(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<Option<BackendUpdate>, BackendError> {
    let normalized = normalize_asdf_version(current_version);

    check_github_backend_update(client, ASDF_GITHUB_REPO, normalized)
        .await
        .map(|opt| opt.map(BackendUpdate::from))
        .map_err(|error| BackendError::network_request_from("asdf update check", error))
}

#[cfg(test)]
mod tests {
    use super::normalize_asdf_version;
    use versi_backend::BackendUpdate;
    use versi_core::{GitHubRelease, backend_update_from_release};

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
        let normalized = normalize_asdf_version("asdf 0.17.0");
        let update: BackendUpdate = backend_update_from_release(release("v0.18.0"), normalized)
            .expect("newer release should produce update metadata")
            .into();

        assert_eq!(update.current_version, "0.17.0");
        assert_eq!(update.latest_version, "0.18.0");
    }

    #[test]
    fn returns_none_when_release_is_not_newer() {
        assert!(backend_update_from_release(release("v0.18.0"), "0.18.0").is_none());
        assert!(backend_update_from_release(release("v0.16.0"), "0.18.0").is_none());
    }

    #[test]
    fn normalize_strips_asdf_prefix() {
        assert_eq!(normalize_asdf_version("asdf 0.17.0"), "0.17.0");
        assert_eq!(normalize_asdf_version("0.17.0"), "0.17.0");
        assert_eq!(normalize_asdf_version("  asdf 0.17.0  "), "0.17.0");
    }
}
