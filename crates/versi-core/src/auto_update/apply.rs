use std::path::Path;

use log::info;

use super::{ApplyResult, AutoUpdateError};

#[cfg(target_os = "macos")]
pub(super) fn apply_update(extract_dir: &Path) -> Result<ApplyResult, AutoUpdateError> {
    let new_app = find_app_bundle(extract_dir)?;
    let current_bundle = current_app_bundle()?;
    let old_bundle = current_bundle.with_extension("app.old");

    info!(
        "Replacing {} with {}",
        current_bundle.display(),
        new_app.display()
    );

    if old_bundle.exists() {
        std::fs::remove_dir_all(&old_bundle).map_err(|error| {
            AutoUpdateError::io_with_path("failed to remove old backup", &old_bundle, &error)
        })?;
    }

    std::fs::rename(&current_bundle, &old_bundle).map_err(|error| {
        AutoUpdateError::io_with_path(
            "failed to move current app bundle aside",
            &current_bundle,
            &error,
        )
    })?;

    match move_dir(&new_app, &current_bundle) {
        Ok(()) => {}
        Err(e) => {
            log::warn!("Apply failed, restoring backup: {e}");
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
fn find_app_bundle(dir: &Path) -> Result<std::path::PathBuf, AutoUpdateError> {
    for entry in std::fs::read_dir(dir)
        .map_err(|error| AutoUpdateError::io_with_path("failed to read extract dir", dir, &error))?
    {
        let entry = entry.map_err(|error| {
            AutoUpdateError::io("failed to read extract directory entry", error)
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("app") && path.is_dir() {
            return Ok(path);
        }
    }
    Err(AutoUpdateError::Invalid(
        "No .app bundle found in extracted archive".to_string(),
    ))
}

#[cfg(target_os = "macos")]
pub(super) fn current_app_bundle() -> Result<std::path::PathBuf, AutoUpdateError> {
    let exe = std::env::current_exe()
        .map_err(|error| AutoUpdateError::io("failed to get current executable", error))?;
    let mut path = exe.as_path();
    loop {
        if path.extension().and_then(|e| e.to_str()) == Some("app") {
            return Ok(path.to_path_buf());
        }
        path = path.parent().ok_or_else(|| {
            AutoUpdateError::Invalid("Current executable is not inside a .app bundle".to_string())
        })?;
    }
}

#[cfg(target_os = "macos")]
fn move_dir(src: &Path, dest: &Path) -> Result<(), AutoUpdateError> {
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }

    copy_dir_recursive(src, dest)?;
    std::fs::remove_dir_all(src).map_err(|error| {
        AutoUpdateError::io_with_path("failed to clean up source directory", src, &error)
    })?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), AutoUpdateError> {
    std::fs::create_dir_all(dest).map_err(|error| {
        AutoUpdateError::io_with_path("failed to create directory", dest, &error)
    })?;

    for entry in std::fs::read_dir(src)
        .map_err(|error| AutoUpdateError::io_with_path("failed to read directory", src, &error))?
    {
        let entry =
            entry.map_err(|error| AutoUpdateError::io("failed to read directory entry", error))?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path).map_err(|error| {
                AutoUpdateError::io(
                    "failed to copy file during update apply",
                    std::io::Error::new(
                        error.kind(),
                        format!("{} -> {}: {error}", src_path.display(), dest_path.display()),
                    ),
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub(super) fn apply_update(extract_dir: &Path) -> Result<ApplyResult, AutoUpdateError> {
    let new_binary = extract_dir.join("versi");
    if !new_binary.exists() {
        return Err(AutoUpdateError::Invalid(
            "No 'versi' binary found in extracted archive".to_string(),
        ));
    }

    let exe = std::env::current_exe()
        .map_err(|error| AutoUpdateError::io("failed to get current executable", error))?;

    info!("Replacing binary via self-replace");
    match self_replace::self_replace(&new_binary) {
        Ok(()) => {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755));
            info!("Linux update applied successfully");
            Ok(ApplyResult::RestartRequired)
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            info!("Permission denied, trying pkexec for elevated replacement");
            apply_update_with_pkexec(&new_binary, &exe)
        }
        Err(error) => Err(AutoUpdateError::io("failed to replace binary", error)),
    }
}

#[cfg(target_os = "linux")]
fn apply_update_with_pkexec(
    new_binary: &Path,
    target: &Path,
) -> Result<ApplyResult, AutoUpdateError> {
    let status = std::process::Command::new("pkexec")
        .args([
            "cp",
            "--",
            &new_binary.to_string_lossy(),
            &target.to_string_lossy(),
        ])
        .status()
        .map_err(|error| AutoUpdateError::io("failed to run pkexec", error))?;

    if !status.success() {
        return Err(AutoUpdateError::Invalid(format!(
            "Elevated update failed. Binary is installed in a system location.\n\
             To update manually, run:\n  sudo cp {} {}",
            new_binary.display(),
            target.display()
        )));
    }

    let _ = std::process::Command::new("pkexec")
        .args(["chmod", "755", &target.to_string_lossy()])
        .status();

    info!("Linux update applied via pkexec");
    Ok(ApplyResult::RestartRequired)
}

#[cfg(target_os = "windows")]
pub(super) fn apply_update(_extract_dir: &Path) -> Result<ApplyResult, AutoUpdateError> {
    unreachable!("Windows uses MSI path, not extract+apply")
}

#[cfg(target_os = "windows")]
pub(super) fn apply_msi(msi_path: &Path) -> Result<ApplyResult, AutoUpdateError> {
    info!("Launching MSI installer: {}", msi_path.display());
    std::process::Command::new("msiexec")
        .args(["/i", &msi_path.to_string_lossy(), "/passive"])
        .spawn()
        .map_err(|error| AutoUpdateError::io("failed to launch MSI installer", error))?;

    Ok(ApplyResult::ExitForInstaller)
}

#[cfg(not(target_os = "windows"))]
pub(super) fn apply_msi(_msi_path: &Path) -> Result<ApplyResult, AutoUpdateError> {
    Err(AutoUpdateError::Invalid(
        "MSI installation is only supported on Windows".to_string(),
    ))
}

#[cfg(target_os = "macos")]
/// Restart the current application bundle.
///
/// # Errors
/// Returns an error if the running app bundle cannot be located or reopened.
pub fn restart_app() -> Result<(), AutoUpdateError> {
    let bundle = current_app_bundle()?;
    std::process::Command::new("open")
        .args(["-n", &bundle.to_string_lossy()])
        .spawn()
        .map_err(|error| AutoUpdateError::io("failed to restart app", error))?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
/// Restart the current executable.
///
/// # Errors
/// Returns an error if the current executable path cannot be resolved or a new
/// process cannot be spawned.
pub fn restart_app() -> Result<(), AutoUpdateError> {
    let exe = std::env::current_exe()
        .map_err(|error| AutoUpdateError::io("failed to get current executable", error))?;

    // On Linux, after self_replace, /proc/self/exe points to the old deleted inode
    // and current_exe() returns a path with " (deleted)" appended.
    // Strip it to get the actual path where the new binary was placed.
    #[cfg(target_os = "linux")]
    let exe = {
        let path_str = exe.to_string_lossy();
        if path_str.ends_with(" (deleted)") {
            let fixed = std::path::PathBuf::from(path_str.trim_end_matches(" (deleted)"));
            info!("Adjusted exe path from deleted inode: {}", fixed.display());
            fixed
        } else {
            exe
        }
    };

    info!("Restarting from: {}", exe.display());
    std::process::Command::new(&exe)
        .spawn()
        .map_err(|error| AutoUpdateError::io("failed to restart app", error))?;
    Ok(())
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

    let Ok(paths) = versi_platform::AppPaths::new() else {
        return;
    };
    let cache_dir = paths.cache_dir;
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && entry.file_name().to_string_lossy().starts_with(".tmp") {
                log::debug!("Cleaning up update temp dir: {}", path.display());
                let _ = std::fs::remove_dir_all(&path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn apply_msi_reports_unsupported_on_non_windows() {
        let result = apply_msi(std::path::Path::new("/tmp/update.msi"));
        assert!(matches!(
            result,
            Err(AutoUpdateError::Invalid(ref message))
                if message == "MSI installation is only supported on Windows"
        ));
    }
}
