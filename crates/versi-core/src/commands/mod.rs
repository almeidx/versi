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
    Some(strip_version_prefix(stdout.trim(), prefix))
}

fn strip_version_prefix(output: &str, prefix: &str) -> String {
    output.strip_prefix(prefix).unwrap_or(output).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_matching_prefix() {
        assert_eq!(strip_version_prefix("fnm 1.2.3", "fnm "), "1.2.3");
    }

    #[test]
    fn strips_matching_prefix_on_pre_trimmed_output() {
        assert_eq!(strip_version_prefix("volta 3.0.0", "volta "), "3.0.0");
    }

    #[test]
    fn returns_output_unchanged_when_leading_whitespace_prevents_prefix_match() {
        assert_eq!(
            strip_version_prefix("  volta 3.0.0", "volta "),
            "  volta 3.0.0"
        );
    }

    #[test]
    fn returns_output_unchanged_when_prefix_does_not_match() {
        assert_eq!(strip_version_prefix("1.2.3", "fnm "), "1.2.3");
    }

    #[test]
    fn returns_empty_string_for_empty_output() {
        assert_eq!(strip_version_prefix("", "fnm "), "");
    }
}
