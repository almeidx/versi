use std::path::Path;

use futures_util::StreamExt;
use log::info;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use super::{AutoUpdateError, UpdateProgress};

pub(super) async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    progress: &mpsc::Sender<UpdateProgress>,
) -> Result<(), AutoUpdateError> {
    let response = client
        .get(url)
        .header("User-Agent", crate::http::USER_AGENT)
        .send()
        .await
        .map_err(|error| AutoUpdateError::http("download request failed", error))?;

    if !response.status().is_success() {
        return Err(AutoUpdateError::Invalid(format!(
            "Download failed with status {}",
            response.status()
        )));
    }

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(dest).await.map_err(|error| {
        AutoUpdateError::io_with_path("failed to create download file", dest, &error)
    })?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| AutoUpdateError::http("download stream error", error))?;
        file.write_all(&chunk).await.map_err(|error| {
            AutoUpdateError::io_with_path("failed to write download data", dest, &error)
        })?;
        downloaded += chunk.len() as u64;
        let _ = progress
            .send(UpdateProgress::Downloading { downloaded, total })
            .await;
    }

    file.flush().await.map_err(|error| {
        AutoUpdateError::io_with_path("failed to flush download file", dest, &error)
    })?;

    info!("Download complete: {downloaded} bytes");
    Ok(())
}
