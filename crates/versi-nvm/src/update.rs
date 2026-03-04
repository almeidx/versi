use versi_backend::{BackendError, BackendUpdate};
use versi_core::check_github_backend_update;

use crate::detection::NvmVariant;

const NVM_UNIX_REPO: &str = "nvm-sh/nvm";
const NVM_WINDOWS_REPO: &str = "coreybutler/nvm-windows";

pub async fn check_for_nvm_update(
    client: &reqwest::Client,
    current_version: &str,
    variant: &NvmVariant,
) -> Result<Option<BackendUpdate>, BackendError> {
    let repo = match variant {
        NvmVariant::Unix | NvmVariant::NotFound => NVM_UNIX_REPO,
        NvmVariant::Windows => NVM_WINDOWS_REPO,
    };

    check_github_backend_update(client, repo, current_version)
        .await
        .map(|opt| opt.map(BackendUpdate::from))
        .map_err(|error| BackendError::network_request_from("nvm update check", error))
}

#[cfg(test)]
mod tests {
    use versi_core::is_newer_version;

    #[test]
    fn newer_version_returns_true() {
        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(is_newer_version("1.1.0", "1.0.9"));
    }

    #[test]
    fn older_version_returns_false() {
        assert!(!is_newer_version("1.0.0", "1.0.1"));
        assert!(!is_newer_version("1.9.9", "2.0.0"));
    }

    #[test]
    fn same_version_returns_false() {
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("0.40.1", "0.40.1"));
    }

    #[test]
    fn two_part_versions() {
        assert!(is_newer_version("1.2", "1.1"));
        assert!(!is_newer_version("1.1", "1.2"));
        assert!(!is_newer_version("1.1", "1.1"));
    }

    #[test]
    fn one_part_versions() {
        assert!(is_newer_version("2", "1"));
        assert!(!is_newer_version("1", "2"));
        assert!(!is_newer_version("1", "1"));
    }

    #[test]
    fn v_prefix_not_stripped_by_function() {
        assert!(is_newer_version("v2.0.0", "v1.0.0"));
    }
}
