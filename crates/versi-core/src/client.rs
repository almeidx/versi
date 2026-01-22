use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::commands::HideWindow;

use crate::error::FnmError;
use crate::progress::{parse_progress_line, InstallProgress};
use crate::version::{
    parse_installed_versions, parse_remote_versions, InstalledVersion, NodeVersion, RemoteVersion,
};

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Wsl { distro: String, shell: String },
}

#[derive(Clone)]
pub struct FnmClient {
    fnm_path: PathBuf,
    fnm_dir: Option<PathBuf>,
    node_dist_mirror: Option<String>,
    environment: Environment,
}

impl FnmClient {
    pub fn new(fnm_path: PathBuf) -> Self {
        Self {
            fnm_path,
            fnm_dir: None,
            node_dist_mirror: None,
            environment: Environment::Native,
        }
    }

    pub fn with_fnm_dir(mut self, dir: PathBuf) -> Self {
        self.fnm_dir = Some(dir);
        self
    }

    pub fn with_node_dist_mirror(mut self, mirror: String) -> Self {
        self.node_dist_mirror = Some(mirror);
        self
    }

    pub fn with_wsl(distro: String, shell: String) -> Self {
        Self {
            fnm_path: PathBuf::from("fnm"),
            fnm_dir: None,
            node_dist_mirror: None,
            environment: Environment::Wsl { distro, shell },
        }
    }

    pub async fn list_installed(&self) -> Result<Vec<InstalledVersion>, FnmError> {
        let output = self.execute(&["list"]).await?;
        Ok(parse_installed_versions(&output))
    }

    pub async fn list_remote(&self) -> Result<Vec<RemoteVersion>, FnmError> {
        let output = self.execute(&["list-remote"]).await?;
        Ok(parse_remote_versions(&output))
    }

    pub async fn list_remote_lts(&self) -> Result<Vec<RemoteVersion>, FnmError> {
        let output = self.execute(&["list-remote", "--lts"]).await?;
        Ok(parse_remote_versions(&output))
    }

    pub async fn current(&self) -> Result<Option<NodeVersion>, FnmError> {
        let output = self.execute(&["current"]).await?;
        let output = output.trim();

        if output.is_empty() || output == "none" || output == "system" {
            return Ok(None);
        }

        output
            .parse()
            .map(Some)
            .map_err(|e: crate::version::VersionParseError| FnmError::ParseError(e.to_string()))
    }

    pub async fn default_version(&self) -> Result<Option<NodeVersion>, FnmError> {
        let versions = self.list_installed().await?;
        Ok(versions
            .into_iter()
            .find(|v| v.is_default)
            .map(|v| v.version))
    }

    pub async fn install(&self, version: &str) -> Result<(), FnmError> {
        self.execute(&["install", version]).await?;
        Ok(())
    }

    pub async fn install_with_progress(
        &self,
        version: &str,
    ) -> Result<mpsc::UnboundedReceiver<InstallProgress>, FnmError> {
        let (tx, rx) = mpsc::unbounded_channel();

        let mut cmd = self.build_command(&["install", version, "--progress", "never"]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| FnmError::IoError("Failed to capture stdout".to_string()))?;

        let tx_stdout = tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();

            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(progress) = parse_progress_line(&line) {
                    let _ = tx_stdout.send(progress);
                }
            }
        });

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| FnmError::IoError("Failed to capture stderr".to_string()))?;

        let tx_stderr = tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();

            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(progress) = parse_progress_line(&line) {
                    let _ = tx_stderr.send(progress);
                }
            }
        });

        let tx_final = tx;
        tokio::spawn(async move {
            let status = child.wait().await;
            match status {
                Ok(s) if s.success() => {
                    let _ = tx_final.send(InstallProgress {
                        phase: crate::InstallPhase::Complete,
                        percent: Some(100.0),
                        ..Default::default()
                    });
                }
                _ => {
                    let _ = tx_final.send(InstallProgress {
                        phase: crate::InstallPhase::Failed,
                        ..Default::default()
                    });
                }
            }
        });

        Ok(rx)
    }

    pub async fn uninstall(&self, version: &str) -> Result<(), FnmError> {
        self.execute(&["uninstall", version]).await?;
        Ok(())
    }

    pub async fn set_default(&self, version: &str) -> Result<(), FnmError> {
        self.execute(&["default", version]).await?;
        Ok(())
    }

    pub async fn use_version(&self, version: &str) -> Result<(), FnmError> {
        self.execute(&["use", version]).await?;
        Ok(())
    }

    pub async fn env_output(&self, shell: &str) -> Result<String, FnmError> {
        self.execute(&["env", "--shell", shell]).await
    }

    fn build_command(&self, args: &[&str]) -> Command {
        match &self.environment {
            Environment::Native => {
                let mut cmd = Command::new(&self.fnm_path);
                cmd.args(args);

                if let Some(dir) = &self.fnm_dir {
                    cmd.env("FNM_DIR", dir);
                }

                if let Some(mirror) = &self.node_dist_mirror {
                    cmd.env("FNM_NODE_DIST_MIRROR", mirror);
                }

                cmd.hide_window();
                cmd
            }
            Environment::Wsl { distro, shell } => {
                let mut cmd = Command::new("wsl.exe");
                let fnm_args = args.join(" ");
                let shell_cmd = format!("fnm {}", fnm_args);
                cmd.args(["-d", distro, "--", shell, "-l", "-c", &shell_cmd]);
                cmd.hide_window();
                cmd
            }
        }
    }

    async fn execute(&self, args: &[&str]) -> Result<String, FnmError> {
        let output = self.build_command(args).output().await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(FnmError::CommandFailed {
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}
