mod apply;
mod download;
mod extract;

use std::path::Path;

use log::info;
use thiserror::Error;
use tokio::sync::mpsc;

pub use apply::{cleanup_old_app_bundle, restart_app};

#[derive(Debug, Clone)]
pub enum UpdateProgress {
    Downloading { downloaded: u64, total: u64 },
    Extracting,
    Applying,
    Complete(ApplyResult),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyResult {
    RestartRequired,
    ExitForInstaller,
}

#[derive(Debug, Error)]
pub enum AutoUpdateError {
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("{context}: {source}")]
    Http {
        context: &'static str,
        #[source]
        source: reqwest::Error,
    },
    #[error("{context}: {source}")]
    Zip {
        context: &'static str,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("{context}: {details}")]
    Platform {
        context: &'static str,
        details: String,
    },
    #[error("{0}")]
    Invalid(String),
}

impl AutoUpdateError {
    pub(crate) fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }

    pub(crate) fn http(context: &'static str, source: reqwest::Error) -> Self {
        Self::Http { context, source }
    }

    pub(crate) fn zip(context: &'static str, source: zip::result::ZipError) -> Self {
        Self::Zip { context, source }
    }

    pub(crate) fn platform(context: &'static str, details: String) -> Self {
        Self::Platform { context, details }
    }

    pub(crate) fn io_with_path(
        context: &'static str,
        path: &Path,
        source: &std::io::Error,
    ) -> Self {
        Self::io(
            context,
            std::io::Error::new(source.kind(), format!("{}: {source}", path.display())),
        )
    }
}

fn sanitize_asset_name(download_url: &str) -> &str {
    let raw_name = download_url.rsplit('/').next().unwrap_or("update-download");
    Path::new(raw_name)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty() && !n.contains(".."))
        .unwrap_or("update-download")
}

fn is_msi_asset(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("msi"))
}

/// Download and apply a packaged Versi update.
///
/// # Errors
/// Returns an error when downloading, extracting, or applying the update fails.
pub async fn download_and_apply(
    client: &reqwest::Client,
    download_url: &str,
    expected_sha256: Option<&str>,
    progress: mpsc::Sender<UpdateProgress>,
) -> Result<ApplyResult, AutoUpdateError> {
    let cache_dir = versi_platform::AppPaths::new()
        .map_err(|error| {
            AutoUpdateError::platform("failed to resolve app paths", error.to_string())
        })?
        .cache_dir;
    std::fs::create_dir_all(&cache_dir)
        .map_err(|error| AutoUpdateError::io("failed to create cache directory", error))?;

    let temp_dir = tempfile::tempdir_in(&cache_dir)
        .map_err(|error| AutoUpdateError::io("failed to create temp directory", error))?;

    let file_name = sanitize_asset_name(download_url);
    let download_path = temp_dir.path().join(file_name);

    info!("Downloading update from {download_url}");
    download::download_file(client, download_url, &download_path, &progress).await?;
    extract::verify_download_checksum(expected_sha256, file_name, &download_path)?;

    let is_msi = is_msi_asset(file_name);

    if is_msi {
        let _ = progress.send(UpdateProgress::Applying).await;
        let _ = temp_dir.keep();
        return apply::apply_msi(&download_path);
    }

    let _ = progress.send(UpdateProgress::Extracting).await;
    let extract_dir = temp_dir.path().join("extracted");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|error| AutoUpdateError::io("failed to create extraction directory", error))?;
    extract::extract_zip(&download_path, &extract_dir)?;

    let _ = progress.send(UpdateProgress::Applying).await;
    apply::apply_update(&extract_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_asset_name_extracts_filename_from_url() {
        assert_eq!(
            sanitize_asset_name("https://github.com/releases/download/v1.0/versi-macos.zip"),
            "versi-macos.zip"
        );
    }

    #[test]
    fn sanitize_asset_name_falls_back_for_empty_url() {
        assert_eq!(sanitize_asset_name(""), "update-download");
    }

    #[test]
    fn sanitize_asset_name_falls_back_for_trailing_slash() {
        assert_eq!(
            sanitize_asset_name("https://example.com/"),
            "update-download"
        );
    }

    #[test]
    fn sanitize_asset_name_rejects_path_traversal() {
        assert_eq!(
            sanitize_asset_name("https://evil.com/.."),
            "update-download"
        );
    }

    #[test]
    fn sanitize_asset_name_handles_bare_filename() {
        assert_eq!(sanitize_asset_name("update.zip"), "update.zip");
    }

    #[test]
    fn is_msi_asset_detects_msi_extension() {
        assert!(is_msi_asset("versi-setup.msi"));
        assert!(is_msi_asset("VERSI.MSI"));
        assert!(is_msi_asset("versi.Msi"));
    }

    #[test]
    fn is_msi_asset_rejects_non_msi() {
        assert!(!is_msi_asset("versi-macos.zip"));
        assert!(!is_msi_asset("versi.tar.gz"));
        assert!(!is_msi_asset("versi"));
        assert!(!is_msi_asset("msi"));
    }

    #[test]
    fn error_io_preserves_context() {
        let source = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err = AutoUpdateError::io("reading config", source);
        let msg = err.to_string();
        assert!(msg.contains("reading config"));
        assert!(msg.contains("gone"));
    }

    #[test]
    fn error_io_with_path_includes_path_in_message() {
        let source = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err =
            AutoUpdateError::io_with_path("open file", std::path::Path::new("/tmp/test"), &source);
        let msg = err.to_string();
        assert!(msg.contains("open file"));
        assert!(msg.contains("/tmp/test"));
    }

    #[test]
    fn error_platform_preserves_context_and_details() {
        let err = AutoUpdateError::platform("resolve paths", "HOME not set".to_string());
        let msg = err.to_string();
        assert!(msg.contains("resolve paths"));
        assert!(msg.contains("HOME not set"));
    }

    #[test]
    fn error_invalid_displays_inner_message() {
        let err = AutoUpdateError::Invalid("bad archive".to_string());
        assert_eq!(err.to_string(), "bad archive");
    }
}
