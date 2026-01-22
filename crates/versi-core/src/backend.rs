use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::commands::HideWindow;

use versi_backend::{
    BackendError, BackendInfo, InstallPhase, InstallProgress, InstalledVersion,
    ManagerCapabilities, NodeVersion, RemoteVersion, ShellInitOptions, VersionManager,
};

use crate::progress::parse_progress_line;
use crate::version::{parse_installed_versions, parse_remote_versions};

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Wsl { distro: String },
}

#[derive(Clone)]
pub struct FnmBackend {
    info: BackendInfo,
    fnm_dir: Option<PathBuf>,
    node_dist_mirror: Option<String>,
    environment: Environment,
}

impl FnmBackend {
    pub fn new(path: PathBuf, version: Option<String>, fnm_dir: Option<PathBuf>) -> Self {
        Self {
            info: BackendInfo {
                name: "fnm",
                path,
                version,
                data_dir: fnm_dir.clone(),
                in_path: true,
            },
            fnm_dir,
            node_dist_mirror: None,
            environment: Environment::Native,
        }
    }

    pub fn with_fnm_dir(mut self, dir: PathBuf) -> Self {
        self.fnm_dir = Some(dir.clone());
        self.info.data_dir = Some(dir);
        self
    }

    pub fn with_node_dist_mirror(mut self, mirror: String) -> Self {
        self.node_dist_mirror = Some(mirror);
        self
    }

    pub fn with_wsl(distro: String) -> Self {
        Self {
            info: BackendInfo {
                name: "fnm",
                path: PathBuf::from("fnm"),
                version: None,
                data_dir: None,
                in_path: true,
            },
            fnm_dir: None,
            node_dist_mirror: None,
            environment: Environment::Wsl { distro },
        }
    }

    fn build_command(&self, args: &[&str]) -> Command {
        match &self.environment {
            Environment::Native => {
                let mut cmd = Command::new(&self.info.path);
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
            Environment::Wsl { distro } => {
                let mut cmd = Command::new("wsl.exe");
                cmd.args(["-d", distro, "fnm"]);
                cmd.args(args);
                cmd.hide_window();
                cmd
            }
        }
    }

    async fn execute(&self, args: &[&str]) -> Result<String, BackendError> {
        let output = self.build_command(args).output().await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(BackendError::CommandFailed {
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}

#[async_trait]
impl VersionManager for FnmBackend {
    fn name(&self) -> &'static str {
        "fnm"
    }

    fn capabilities(&self) -> ManagerCapabilities {
        ManagerCapabilities {
            supports_progress: true,
            supports_lts_filter: true,
            supports_use_version: true,
            supports_shell_integration: true,
            supports_auto_switch: true,
            supports_corepack: true,
            supports_resolve_engines: true,
        }
    }

    fn backend_info(&self) -> &BackendInfo {
        &self.info
    }

    async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError> {
        let output = self.execute(&["list"]).await?;
        Ok(parse_installed_versions(&output))
    }

    async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        let output = self.execute(&["list-remote"]).await?;
        Ok(parse_remote_versions(&output))
    }

    async fn list_remote_lts(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        let output = self.execute(&["list-remote", "--lts"]).await?;
        Ok(parse_remote_versions(&output))
    }

    async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        let output = self.execute(&["current"]).await?;
        let output = output.trim();

        if output.is_empty() || output == "none" || output == "system" {
            return Ok(None);
        }

        output
            .parse()
            .map(Some)
            .map_err(|e: versi_backend::VersionParseError| BackendError::ParseError(e.to_string()))
    }

    async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        let versions = self.list_installed().await?;
        Ok(versions
            .into_iter()
            .find(|v| v.is_default)
            .map(|v| v.version))
    }

    async fn install(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["install", version]).await?;
        Ok(())
    }

    async fn install_with_progress(
        &self,
        version: &str,
    ) -> Result<mpsc::UnboundedReceiver<InstallProgress>, BackendError> {
        let (tx, rx) = mpsc::unbounded_channel();

        let mut cmd = self.build_command(&["install", version, "--progress", "never"]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BackendError::IoError("Failed to capture stdout".to_string()))?;

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
            .ok_or_else(|| BackendError::IoError("Failed to capture stderr".to_string()))?;

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
                        phase: InstallPhase::Complete,
                        percent: Some(100.0),
                        ..Default::default()
                    });
                }
                _ => {
                    let _ = tx_final.send(InstallProgress {
                        phase: InstallPhase::Failed,
                        ..Default::default()
                    });
                }
            }
        });

        Ok(rx)
    }

    async fn uninstall(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["uninstall", version]).await?;
        Ok(())
    }

    async fn set_default(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["default", version]).await?;
        Ok(())
    }

    async fn use_version(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["use", version]).await?;
        Ok(())
    }

    fn shell_init_command(&self, shell: &str, options: &ShellInitOptions) -> Option<String> {
        let mut flags = Vec::new();

        if options.use_on_cd {
            flags.push("--use-on-cd");
        }
        if options.resolve_engines {
            flags.push("--resolve-engines");
        }
        if options.corepack_enabled {
            flags.push("--corepack-enabled");
        }

        let flags_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" {}", flags.join(" "))
        };

        match shell {
            "bash" | "zsh" => Some(format!("eval \"$(fnm env{})\"", flags_str)),
            "fish" => Some(format!("fnm env{} | source", flags_str)),
            "powershell" | "pwsh" => Some(format!(
                "fnm env{} | Out-String | Invoke-Expression",
                flags_str
            )),
            _ => None,
        }
    }
}
