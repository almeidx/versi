use std::path::PathBuf;
use tokio::process::Command;

use crate::version::{
    parse_unix_installed, parse_unix_remote, parse_windows_installed, parse_windows_remote,
};
use versi_backend::{
    BackendError, InstalledVersion, NodeVersion, RemoteVersion, command_output_to_result,
    find_default_version, parse_current_version, sanitize_terminal_text,
};
use versi_platform::HideWindow;

#[derive(Debug, Clone)]
pub enum NvmEnvironment {
    Unix { nvm_dir: PathBuf },
    Windows { nvm_exe: PathBuf },
    Wsl { distro: String, nvm_dir: String },
}

#[derive(Debug, Clone)]
pub struct NvmClient {
    pub environment: NvmEnvironment,
}

impl NvmClient {
    #[must_use]
    pub fn unix(nvm_dir: PathBuf) -> Self {
        Self {
            environment: NvmEnvironment::Unix { nvm_dir },
        }
    }

    #[must_use]
    pub fn windows(nvm_exe: PathBuf) -> Self {
        Self {
            environment: NvmEnvironment::Windows { nvm_exe },
        }
    }

    #[must_use]
    pub fn wsl(distro: String, nvm_dir: String) -> Self {
        Self {
            environment: NvmEnvironment::Wsl { distro, nvm_dir },
        }
    }

    #[must_use]
    pub fn is_windows(&self) -> bool {
        matches!(self.environment, NvmEnvironment::Windows { .. })
    }

    fn build_nvm_command(&self, nvm_args: &[&str]) -> Command {
        match &self.environment {
            NvmEnvironment::Unix { nvm_dir } => {
                let script = "[ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\"; nvm \"$@\"";
                let mut cmd = Command::new("bash");
                cmd.args(["-c", script, "bash"]);
                cmd.args(nvm_args);
                cmd.env("NVM_DIR", nvm_dir);
                cmd.env("TERM", "dumb");
                cmd.env("NO_COLOR", "1");
                cmd.hide_window();
                cmd
            }
            NvmEnvironment::Windows { nvm_exe } => {
                let mut cmd = Command::new(nvm_exe);
                cmd.args(nvm_args);
                cmd.hide_window();
                cmd
            }
            NvmEnvironment::Wsl { distro, nvm_dir } => {
                let script = "NVM_DIR=\"$1\"; export NVM_DIR; [ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\"; shift; nvm \"$@\"";
                let mut cmd = Command::new("wsl.exe");
                cmd.args(["-d", distro, "--", "bash", "-c", script, "bash", nvm_dir]);
                cmd.args(nvm_args);
                cmd.hide_window();
                cmd
            }
        }
    }

    async fn execute(&self, nvm_args: &[&str]) -> Result<String, BackendError> {
        let output = self.build_nvm_command(nvm_args).output().await?;
        command_output_to_result(&output).map(|stdout| sanitize_terminal_text(&stdout))
    }

    /// List installed Node.js versions managed by this `nvm` environment.
    ///
    /// # Errors
    /// Returns an error if invoking `nvm list` fails.
    pub async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError> {
        let output = self.execute(&["list"]).await?;
        Ok(if self.is_windows() {
            parse_windows_installed(&output)
        } else {
            parse_unix_installed(&output)
        })
    }

    /// List remote Node.js versions available for installation.
    ///
    /// # Errors
    /// Returns an error if invoking the remote listing command fails.
    pub async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        if self.is_windows() {
            let output = self.execute(&["list", "available"]).await?;
            Ok(parse_windows_remote(&output))
        } else {
            let output = self.execute(&["ls-remote"]).await?;
            Ok(parse_unix_remote(&output))
        }
    }

    /// List remote LTS Node.js versions available for installation.
    ///
    /// # Errors
    /// Returns an error if remote version listing fails.
    pub async fn list_remote_lts(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        if self.is_windows() {
            let all = self.list_remote().await?;
            Ok(all
                .into_iter()
                .filter(|v| v.lts_codename.is_some())
                .collect())
        } else {
            let output = self.execute(&["ls-remote", "--lts"]).await?;
            Ok(parse_unix_remote(&output))
        }
    }

    /// Return the currently active Node.js version.
    ///
    /// # Errors
    /// Returns an error if the command fails or the version output is invalid.
    pub async fn current(&self) -> Result<Option<NodeVersion>, BackendError> {
        let output = self.execute(&["current"]).await?;
        parse_current_version(&output)
    }

    /// Return the configured default Node.js version, if any.
    ///
    /// # Errors
    /// Returns an error if querying installed/default versions fails.
    pub async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        if self.is_windows() {
            let versions = self.list_installed().await?;
            Ok(find_default_version(versions))
        } else {
            let output = self.execute(&["alias", "default"]).await;
            match output {
                Ok(text) => {
                    let trimmed = text.trim();
                    let version_part = trimmed
                        .split("->")
                        .last()
                        .unwrap_or(trimmed)
                        .trim()
                        .trim_start_matches('v');
                    let version_str = version_part
                        .split(|c: char| !c.is_ascii_digit() && c != '.')
                        .next()
                        .unwrap_or("");
                    if version_str.is_empty() {
                        Ok(None)
                    } else {
                        version_str.parse().map(Some).map_err(BackendError::from)
                    }
                }
                Err(e) => {
                    if is_missing_default_alias_error(&e) {
                        log::debug!("nvm alias default missing, assuming no default: {e}");
                        Ok(None)
                    } else {
                        Err(e)
                    }
                }
            }
        }
    }

    /// Install a Node.js version.
    ///
    /// # Errors
    /// Returns an error if the install command fails.
    pub async fn install(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["install", version]).await?;
        Ok(())
    }

    /// Uninstall a Node.js version.
    ///
    /// # Errors
    /// Returns an error if the uninstall command fails.
    pub async fn uninstall(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["uninstall", version]).await?;
        Ok(())
    }

    /// Set the default Node.js version.
    ///
    /// # Errors
    /// Returns an error if the platform-specific default-setting command fails.
    pub async fn set_default(&self, version: &str) -> Result<(), BackendError> {
        if self.is_windows() {
            self.execute(&["use", version]).await?;
        } else {
            self.execute(&["alias", "default", version]).await?;
        }
        Ok(())
    }

    /// Activate a Node.js version for the current shell context.
    ///
    /// # Errors
    /// Returns an error if the `nvm use` command fails.
    pub async fn use_version(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["use", version]).await?;
        Ok(())
    }

    /// Return the installed `nvm` tool version string.
    ///
    /// # Errors
    /// Returns an error if querying `nvm --version` fails.
    pub async fn version(&self) -> Result<String, BackendError> {
        if self.is_windows() {
            let output = self.execute(&["version"]).await?;
            Ok(output.trim().to_string())
        } else {
            let output = self.execute(&["--version"]).await?;
            Ok(output.trim().to_string())
        }
    }
}

fn is_missing_default_alias_error(error: &BackendError) -> bool {
    match error {
        BackendError::CommandFailed { stderr } => is_missing_default_alias_message(stderr),
        _ => false,
    }
}

fn is_missing_default_alias_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("default -> n/a")
        || lower.contains("alias default -> n/a")
        || lower.contains("default is not set")
        || lower.contains("default alias is not set")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_windows_returns_true_for_windows_environment() {
        let client = NvmClient::windows(PathBuf::from("C:\\nvm\\nvm.exe"));
        assert!(client.is_windows());
    }

    #[test]
    fn is_windows_returns_false_for_unix_environment() {
        let client = NvmClient::unix(PathBuf::from("/home/user/.nvm"));
        assert!(!client.is_windows());
    }

    #[test]
    fn is_windows_returns_false_for_wsl_environment() {
        let client = NvmClient::wsl("Ubuntu".to_string(), "/home/user/.nvm".to_string());
        assert!(!client.is_windows());
    }

    #[test]
    fn wsl_constructor_sets_environment() {
        let client = NvmClient::wsl("Debian".to_string(), "/home/user/.nvm".to_string());
        assert!(matches!(
            client.environment,
            NvmEnvironment::Wsl { ref distro, ref nvm_dir }
            if distro == "Debian" && nvm_dir == "/home/user/.nvm"
        ));
    }

    #[test]
    fn missing_default_alias_error_detection_accepts_known_patterns() {
        let missing = BackendError::CommandFailed {
            stderr: "alias default -> N/A".to_string(),
        };

        assert!(is_missing_default_alias_error(&missing));
    }

    #[test]
    fn missing_default_alias_error_detection_rejects_unrelated_failures() {
        let unrelated = BackendError::CommandFailed {
            stderr: "nvm: command not found".to_string(),
        };

        assert!(!is_missing_default_alias_error(&unrelated));
    }
}
