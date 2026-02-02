use async_trait::async_trait;
use log::{debug, error, info, trace};
use std::path::PathBuf;
use tokio::process::Command;

use versi_core::HideWindow;

use versi_backend::{
    BackendError, BackendInfo, InstalledVersion, ManagerCapabilities, NodeVersion, RemoteVersion,
    ShellInitOptions, VersionManager,
};

use crate::version::{parse_installed_versions, parse_remote_versions};

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Wsl { distro: String, fnm_path: String },
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

    pub fn with_wsl(distro: String, fnm_path: String) -> Self {
        Self {
            info: BackendInfo {
                name: "fnm",
                path: PathBuf::from(&fnm_path),
                version: None,
                data_dir: None,
                in_path: true,
            },
            fnm_dir: None,
            node_dist_mirror: None,
            environment: Environment::Wsl { distro, fnm_path },
        }
    }

    fn build_command(&self, args: &[&str]) -> Command {
        match &self.environment {
            Environment::Native => {
                debug!(
                    "Building native fnm command: {:?} {}",
                    self.info.path,
                    args.join(" ")
                );

                let mut cmd = Command::new(&self.info.path);
                cmd.args(args);

                if let Some(dir) = &self.fnm_dir {
                    debug!("Setting FNM_DIR={:?}", dir);
                    cmd.env("FNM_DIR", dir);
                }

                if let Some(mirror) = &self.node_dist_mirror {
                    debug!("Setting FNM_NODE_DIST_MIRROR={}", mirror);
                    cmd.env("FNM_NODE_DIST_MIRROR", mirror);
                }

                cmd.hide_window();
                cmd
            }
            Environment::Wsl { distro, fnm_path } => {
                debug!(
                    "Building WSL fnm command: wsl.exe -d {} -- {} {}",
                    distro,
                    fnm_path,
                    args.join(" ")
                );

                let mut cmd = Command::new("wsl.exe");
                cmd.args(["-d", distro, "--", fnm_path]);
                cmd.args(args);
                cmd.hide_window();
                cmd
            }
        }
    }

    async fn execute(&self, args: &[&str]) -> Result<String, BackendError> {
        info!("Executing fnm command: {}", args.join(" "));

        let output = self.build_command(args).output().await?;

        debug!("fnm command exit status: {:?}", output.status);
        trace!("fnm stdout: {}", String::from_utf8_lossy(&output.stdout));

        if !output.stderr.is_empty() {
            trace!("fnm stderr: {}", String::from_utf8_lossy(&output.stderr));
        }

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            debug!("fnm command succeeded, output: {} bytes", stdout.len());
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            error!("fnm command failed: args={:?}, stderr='{}'", args, stderr);
            Err(BackendError::CommandFailed { stderr })
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
