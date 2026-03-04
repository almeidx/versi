use std::path::Path;

use tokio::process::Command;
pub use versi_platform::HideWindow;

pub async fn get_cli_version(path: &Path, prefix: &str) -> Option<String> {
    let output = match Command::new(path)
        .arg("--version")
        .hide_window()
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) => {
            log::debug!(
                "Failed to run {} --version at {}: {e}",
                prefix.trim(),
                path.display()
            );
            return None;
        }
    };

    if !output.status.success() {
        log::debug!(
            "{} --version exited with {:?} at {}",
            prefix.trim(),
            output.status.code(),
            path.display()
        );
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout
        .trim()
        .strip_prefix(prefix)
        .unwrap_or(stdout.trim())
        .to_string();

    Some(version)
}
