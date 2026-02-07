use std::path::PathBuf;
use tokio::process::Command;

use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion};
use versi_platform::HideWindow;

use crate::error::NvmError;
use crate::version::{
    clean_output, parse_unix_installed, parse_unix_remote, parse_windows_installed,
    parse_windows_remote,
};

#[derive(Debug, Clone)]
pub enum NvmEnvironment {
    Unix { nvm_dir: PathBuf },
    Windows { nvm_exe: PathBuf },
    Wsl { distro: String, nvm_dir: String },
}

#[derive(Clone)]
pub struct NvmClient {
    pub environment: NvmEnvironment,
}

impl NvmClient {
    pub fn unix(nvm_dir: PathBuf) -> Self {
        Self {
            environment: NvmEnvironment::Unix { nvm_dir },
        }
    }

    pub fn windows(nvm_exe: PathBuf) -> Self {
        Self {
            environment: NvmEnvironment::Windows { nvm_exe },
        }
    }

    pub fn wsl(distro: String, nvm_dir: String) -> Self {
        Self {
            environment: NvmEnvironment::Wsl { distro, nvm_dir },
        }
    }

    pub fn is_windows(&self) -> bool {
        matches!(self.environment, NvmEnvironment::Windows { .. })
    }

    fn build_nvm_command(&self, nvm_args: &str) -> Command {
        match &self.environment {
            NvmEnvironment::Unix { nvm_dir } => {
                let script = format!(
                    "export NVM_DIR=\"{}\"; [ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\"; {}",
                    nvm_dir.display(),
                    nvm_args
                );
                let mut cmd = Command::new("bash");
                cmd.args(["-c", &script]);
                cmd.env("TERM", "dumb");
                cmd.env("NO_COLOR", "1");
                cmd.hide_window();
                cmd
            }
            NvmEnvironment::Windows { nvm_exe } => {
                let parts: Vec<&str> = nvm_args.split_whitespace().collect();
                let (_, args) = if !parts.is_empty() && parts[0] == "nvm" {
                    ("nvm", &parts[1..])
                } else {
                    ("nvm", parts.as_slice())
                };
                let mut cmd = Command::new(nvm_exe);
                cmd.args(args);
                cmd.hide_window();
                cmd
            }
            NvmEnvironment::Wsl { distro, nvm_dir } => {
                let script = format!(
                    "export NVM_DIR=\"{}\"; [ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\"; {}",
                    nvm_dir, nvm_args
                );
                let mut cmd = Command::new("wsl.exe");
                cmd.args(["-d", distro, "--", "bash", "-c", &script]);
                cmd.hide_window();
                cmd
            }
        }
    }

    async fn execute(&self, nvm_args: &str) -> Result<String, NvmError> {
        let output = self.build_nvm_command(nvm_args).output().await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(clean_output(&stdout))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(NvmError::CommandFailed { stderr })
        }
    }

    pub async fn list_installed(&self) -> Result<Vec<InstalledVersion>, NvmError> {
        let output = self.execute("nvm list").await?;
        Ok(if self.is_windows() {
            parse_windows_installed(&output)
        } else {
            parse_unix_installed(&output)
        })
    }

    pub async fn list_remote(&self) -> Result<Vec<RemoteVersion>, NvmError> {
        if self.is_windows() {
            let output = self.execute("nvm list available").await?;
            Ok(parse_windows_remote(&output))
        } else {
            let output = self.execute("nvm ls-remote").await?;
            Ok(parse_unix_remote(&output))
        }
    }

    pub async fn list_remote_lts(&self) -> Result<Vec<RemoteVersion>, NvmError> {
        if self.is_windows() {
            let all = self.list_remote().await?;
            Ok(all
                .into_iter()
                .filter(|v| v.lts_codename.is_some())
                .collect())
        } else {
            let output = self.execute("nvm ls-remote --lts").await?;
            Ok(parse_unix_remote(&output))
        }
    }

    pub async fn current(&self) -> Result<Option<NodeVersion>, NvmError> {
        let output = self.execute("nvm current").await?;
        let output = output.trim().trim_start_matches('v');

        if output.is_empty() || output == "none" || output == "system" {
            return Ok(None);
        }

        output
            .parse()
            .map(Some)
            .map_err(|e: versi_backend::VersionParseError| NvmError::ParseError(e.to_string()))
    }

    pub async fn default_version(&self) -> Result<Option<NodeVersion>, NvmError> {
        if self.is_windows() {
            let versions = self.list_installed().await?;
            Ok(versions
                .into_iter()
                .find(|v| v.is_default)
                .map(|v| v.version))
        } else {
            let output = self.execute("nvm alias default").await;
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
                        version_str.parse().map(Some).map_err(
                            |e: versi_backend::VersionParseError| {
                                NvmError::ParseError(e.to_string())
                            },
                        )
                    }
                }
                Err(_) => Ok(None),
            }
        }
    }

    pub async fn install(&self, version: &str) -> Result<(), NvmError> {
        self.execute(&format!("nvm install {}", version)).await?;
        Ok(())
    }

    pub async fn uninstall(&self, version: &str) -> Result<(), NvmError> {
        self.execute(&format!("nvm uninstall {}", version)).await?;
        Ok(())
    }

    pub async fn set_default(&self, version: &str) -> Result<(), NvmError> {
        if self.is_windows() {
            self.execute(&format!("nvm use {}", version)).await?;
        } else {
            self.execute(&format!("nvm alias default {}", version))
                .await?;
        }
        Ok(())
    }

    pub async fn use_version(&self, version: &str) -> Result<(), NvmError> {
        self.execute(&format!("nvm use {}", version)).await?;
        Ok(())
    }

    pub async fn version(&self) -> Result<String, NvmError> {
        if self.is_windows() {
            let output = self.execute("nvm version").await?;
            Ok(output.trim().to_string())
        } else {
            let output = self.execute("nvm --version").await?;
            Ok(output.trim().to_string())
        }
    }
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
}
