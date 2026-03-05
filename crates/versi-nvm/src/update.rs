use versi_backend::{BackendError, BackendUpdate};
use versi_core::check_github_backend_update;

use crate::detection::NvmVariant;

const NVM_UNIX_REPO: &str = "nvm-sh/nvm";
const NVM_WINDOWS_REPO: &str = "coreybutler/nvm-windows";

pub(crate) async fn check_for_nvm_update(
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
