use std::path::Path;

use log::{debug, info, warn};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum UpdateProgress {
    Downloading { downloaded: u64, total: u64 },
    Extracting,
    Applying,
    Complete(ApplyResult),
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum ApplyResult {
    RestartRequired,
    ExitForInstaller,
}

pub async fn download_and_apply(
    client: &reqwest::Client,
    download_url: &str,
    progress: mpsc::Sender<UpdateProgress>,
) -> Result<ApplyResult, String> {
    let cache_dir = versi_platform::AppPaths::new().cache_dir;
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {e}"))?;

    let temp_dir = tempfile::tempdir_in(&cache_dir)
        .map_err(|e| format!("Failed to create temp directory: {e}"))?;

    let file_name = download_url.rsplit('/').next().unwrap_or("update-download");
    let download_path = temp_dir.path().join(file_name);

    info!("Downloading update from {download_url}");
    download_file(client, download_url, &download_path, &progress).await?;

    let is_msi = file_name.ends_with(".msi");

    if is_msi {
        let _ = progress.send(UpdateProgress::Applying).await;
        return apply_msi(&download_path);
    }

    let _ = progress.send(UpdateProgress::Extracting).await;
    let extract_dir = temp_dir.path().join("extracted");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extraction directory: {e}"))?;
    extract_zip(&download_path, &extract_dir)?;

    let _ = progress.send(UpdateProgress::Applying).await;
    apply_update(&extract_dir)
}

async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    progress: &mpsc::Sender<UpdateProgress>,
) -> Result<(), String> {
    use futures_util::StreamExt;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("Failed to create download file: {e}"))?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {e}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write download data: {e}"))?;
        downloaded += chunk.len() as u64;
        let _ = progress
            .send(UpdateProgress::Downloading { downloaded, total })
            .await;
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush download file: {e}"))?;

    info!("Download complete: {} bytes", downloaded);
    Ok(())
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file =
        std::fs::File::open(zip_path).map_err(|e| format!("Failed to open zip file: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {e}"))?;
        let Some(name) = entry.enclosed_name() else {
            warn!("Skipping zip entry with unsafe path");
            continue;
        };
        let out_path = dest.join(name);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create directory {}: {e}", out_path.display()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Failed to create parent directory {}: {e}",
                        parent.display()
                    )
                })?;
            }
            let mut outfile = std::fs::File::create(&out_path)
                .map_err(|e| format!("Failed to create file {}: {e}", out_path.display()))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Failed to extract {}: {e}", out_path.display()))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    let _ =
                        std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode));
                }
            }
        }
    }

    debug!("Extraction complete to {}", dest.display());
    Ok(())
}

#[cfg(target_os = "macos")]
fn apply_update(extract_dir: &Path) -> Result<ApplyResult, String> {
    let new_app = find_app_bundle(extract_dir)?;
    let current_bundle = current_app_bundle()?;
    let old_bundle = current_bundle.with_extension("app.old");

    info!(
        "Replacing {} with {}",
        current_bundle.display(),
        new_app.display()
    );

    if old_bundle.exists() {
        std::fs::remove_dir_all(&old_bundle)
            .map_err(|e| format!("Failed to remove old backup: {e}"))?;
    }

    std::fs::rename(&current_bundle, &old_bundle)
        .map_err(|e| format!("Failed to move current bundle aside: {e}"))?;

    match move_dir(&new_app, &current_bundle) {
        Ok(()) => {}
        Err(e) => {
            warn!("Apply failed, restoring backup: {e}");
            let _ = std::fs::rename(&old_bundle, &current_bundle);
            return Err(e);
        }
    }

    let _ = std::process::Command::new("xattr")
        .args(["-cr", &current_bundle.to_string_lossy()])
        .output();

    info!("macOS update applied successfully");
    Ok(ApplyResult::RestartRequired)
}

#[cfg(target_os = "macos")]
fn find_app_bundle(dir: &Path) -> Result<std::path::PathBuf, String> {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("Failed to read extract dir: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("app") && path.is_dir() {
            return Ok(path);
        }
    }
    Err("No .app bundle found in extracted archive".to_string())
}

#[cfg(target_os = "macos")]
fn current_app_bundle() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get current exe: {e}"))?;
    let mut path = exe.as_path();
    loop {
        if path.extension().and_then(|e| e.to_str()) == Some("app") {
            return Ok(path.to_path_buf());
        }
        path = path
            .parent()
            .ok_or_else(|| "Current executable is not inside a .app bundle".to_string())?;
    }
}

#[cfg(target_os = "macos")]
fn move_dir(src: &Path, dest: &Path) -> Result<(), String> {
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }

    copy_dir_recursive(src, dest)?;
    std::fs::remove_dir_all(src).map_err(|e| format!("Failed to clean up source dir: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest)
        .map_err(|e| format!("Failed to create dir {}: {e}", dest.display()))?;

    for entry in std::fs::read_dir(src).map_err(|e| format!("Failed to read dir: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path).map_err(|e| {
                format!(
                    "Failed to copy {} -> {}: {e}",
                    src_path.display(),
                    dest_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_update(extract_dir: &Path) -> Result<ApplyResult, String> {
    let new_binary = extract_dir.join("versi");
    if !new_binary.exists() {
        return Err("No 'versi' binary found in extracted archive".to_string());
    }

    info!("Replacing binary via self-replace");
    self_replace::self_replace(&new_binary)
        .map_err(|e| format!("Failed to replace binary: {e}"))?;

    use std::os::unix::fs::PermissionsExt;
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get current exe: {e}"))?;
    let _ = std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755));

    info!("Linux update applied successfully");
    Ok(ApplyResult::RestartRequired)
}

#[cfg(target_os = "windows")]
fn apply_update(extract_dir: &Path) -> Result<ApplyResult, String> {
    unreachable!("Windows uses MSI path, not extract+apply")
}

#[cfg(target_os = "windows")]
fn apply_msi(msi_path: &Path) -> Result<ApplyResult, String> {
    info!("Launching MSI installer: {}", msi_path.display());
    std::process::Command::new("msiexec")
        .args(["/i", &msi_path.to_string_lossy(), "/passive"])
        .spawn()
        .map_err(|e| format!("Failed to launch MSI installer: {e}"))?;

    Ok(ApplyResult::ExitForInstaller)
}

#[cfg(not(target_os = "windows"))]
fn apply_msi(_msi_path: &Path) -> Result<ApplyResult, String> {
    Err("MSI installation is only supported on Windows".to_string())
}

pub fn cleanup_old_app_bundle() {
    #[cfg(target_os = "macos")]
    {
        if let Ok(bundle) = current_app_bundle() {
            let old = bundle.with_extension("app.old");
            if old.exists() {
                info!("Cleaning up old app bundle: {}", old.display());
                let _ = std::fs::remove_dir_all(&old);
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub fn restart_app() -> Result<(), String> {
    let bundle = current_app_bundle()?;
    std::process::Command::new("open")
        .args(["-n", &bundle.to_string_lossy()])
        .spawn()
        .map_err(|e| format!("Failed to restart app: {e}"))?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn restart_app() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get current exe: {e}"))?;
    std::process::Command::new(&exe)
        .spawn()
        .map_err(|e| format!("Failed to restart app: {e}"))?;
    Ok(())
}
