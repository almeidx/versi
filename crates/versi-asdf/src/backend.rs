use async_trait::async_trait;
use log::{debug, error, info, trace};
use std::path::PathBuf;
use tokio::process::Command;

use versi_core::HideWindow;

use versi_backend::{
    BackendError, BackendInfo, CommandEnvironment, InstalledVersion, ManagerCapabilities,
    NodeVersion, RemoteVersion, ShellInitOptions, VersionManager, build_backend_command,
    command_output_to_result, strip_version_prefix,
};

use crate::version::{parse_current_version, parse_installed_versions, parse_remote_versions};

#[derive(Debug, Clone)]
pub struct AsdfBackend {
    info: BackendInfo,
    command_env: CommandEnvironment,
}

impl AsdfBackend {
    #[must_use]
    pub fn new(path: PathBuf, version: Option<String>, asdf_data_dir: Option<PathBuf>) -> Self {
        let command_env = CommandEnvironment::Native {
            binary_path: path.clone(),
        };
        Self {
            info: BackendInfo {
                name: "asdf",
                path,
                version,
                data_dir: asdf_data_dir,
                in_path: true,
            },
            command_env,
        }
    }

    #[must_use]
    pub fn with_in_path(mut self, in_path: bool) -> Self {
        self.info.in_path = in_path;
        self
    }

    #[must_use]
    pub fn with_wsl(distro: String, asdf_path: String) -> Self {
        Self {
            info: BackendInfo {
                name: "asdf",
                path: PathBuf::from(&asdf_path),
                version: None,
                data_dir: None,
                in_path: false,
            },
            command_env: CommandEnvironment::Wsl {
                distro,
                binary_path: asdf_path,
            },
        }
    }

    fn apply_native_env(&self, cmd: &mut Command) {
        if !self.command_env.is_native() {
            return;
        }

        if let Some(dir) = &self.info.data_dir {
            debug!("Setting ASDF_DATA_DIR={}", dir.display());
            cmd.env("ASDF_DATA_DIR", dir);
        }
    }

    fn build_command(&self, args: &[&str], home_scope: bool) -> Command {
        match &self.command_env {
            CommandEnvironment::Native { .. } => {
                let mut cmd = build_backend_command(&self.command_env, args);
                self.apply_native_env(&mut cmd);
                if home_scope && let Some(home) = dirs::home_dir() {
                    cmd.current_dir(home);
                }
                cmd
            }
            CommandEnvironment::Wsl {
                distro,
                binary_path,
            } => {
                if home_scope {
                    debug!(
                        "Building WSL asdf home-scope command: {} {}",
                        binary_path,
                        args.join(" "),
                    );

                    let mut cmd = Command::new("wsl.exe");
                    cmd.args([
                        "-d",
                        distro,
                        "--",
                        "sh",
                        "-c",
                        "cd \"$HOME\"; \"$@\"",
                        "sh",
                        binary_path,
                    ]);
                    cmd.args(args);
                    cmd.hide_window();
                    cmd
                } else {
                    build_backend_command(&self.command_env, args)
                }
            }
        }
    }

    async fn execute(&self, args: &[&str], home_scope: bool) -> Result<String, BackendError> {
        debug!(
            "asdf: executing command: {} (home_scope={home_scope})",
            args.join(" "),
        );

        let mut command = self.build_command(args, home_scope);
        command.kill_on_drop(true);
        let output = command.output().await?;

        debug!("asdf command exit status: {:?}", output.status);
        trace!("asdf stdout: {}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            trace!("asdf stderr: {}", String::from_utf8_lossy(&output.stderr));
        }

        command_output_to_result(&output).inspect_err(|err| {
            error!("asdf command failed: args={args:?}, error='{err}'");
        })
    }

    async fn run_current_version(
        &self,
        home_scope: bool,
    ) -> Result<Option<NodeVersion>, BackendError> {
        let args = ["current", "nodejs", "--no-header"];
        let mut command = self.build_command(&args, home_scope);
        command.kill_on_drop(true);
        let output = command.output().await?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if output.status.success() {
            return Ok(parse_current_version(&stdout).or_else(|| parse_current_version(&stderr)));
        }

        let combined = format!("{}\n{}", stderr.trim(), stdout.trim());
        let combined_lower = combined.to_ascii_lowercase();
        if combined_lower.contains("no version")
            || combined_lower.contains("no preset version installed")
            || combined_lower.contains("no compatible versions installed")
            || combined_lower.contains("not installed")
        {
            return Ok(None);
        }

        Err(BackendError::CommandFailed {
            stderr: combined.trim().to_string(),
        })
    }
}

#[async_trait]
impl VersionManager for AsdfBackend {
    fn name(&self) -> &'static str {
        "asdf"
    }

    fn capabilities(&self) -> ManagerCapabilities {
        ManagerCapabilities {
            supports_lts_filter: false,
            supports_use_version: false,
            supports_shell_integration: true,
            supports_auto_switch: false,
            supports_corepack: false,
            supports_resolve_engines: false,
            supports_uninstall: true,
        }
    }

    fn backend_info(&self) -> &BackendInfo {
        &self.info
    }

    async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError> {
        let output = self.execute(&["list", "nodejs"], false).await?;
        let mut versions = parse_installed_versions(&output);

        let default = self.default_version().await?;
        if let Some(default_version) = default {
            for version in &mut versions {
                if version.version == default_version {
                    version.is_default = true;
                }
            }
        }

        Ok(versions)
    }

    async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        let output = self.execute(&["list", "all", "nodejs"], false).await?;
        Ok(parse_remote_versions(&output))
    }

    async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        self.run_current_version(false).await
    }

    async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        self.run_current_version(true).await
    }

    async fn install(&self, version: &str) -> Result<(), BackendError> {
        info!("asdf: installing version {version}");
        let normalized = strip_version_prefix(version);
        self.execute(&["install", "nodejs", normalized], false)
            .await?;
        Ok(())
    }

    async fn uninstall(&self, version: &str) -> Result<(), BackendError> {
        info!("asdf: uninstalling version {version}");
        let normalized = strip_version_prefix(version);
        self.execute(&["uninstall", "nodejs", normalized], false)
            .await?;
        Ok(())
    }

    async fn set_default(&self, version: &str) -> Result<(), BackendError> {
        info!("asdf: setting default version to {version}");
        let normalized = strip_version_prefix(version);
        self.execute(&["set", "-u", "nodejs", normalized], false)
            .await?;
        Ok(())
    }

    fn shell_init_command(&self, shell: &str, _options: &ShellInitOptions) -> Option<String> {
        match shell {
            "bash" | "zsh" => Some(
                "export PATH=\"${ASDF_DATA_DIR:-$HOME/.asdf}/bin:${ASDF_DATA_DIR:-$HOME/.asdf}/shims:$PATH\""
                    .to_string(),
            ),
            "fish" => Some("if test -z \"$ASDF_DATA_DIR\"; set -gx PATH \"$HOME/.asdf/bin\" \"$HOME/.asdf/shims\" $PATH; else; set -gx PATH \"$ASDF_DATA_DIR/bin\" \"$ASDF_DATA_DIR/shims\" $PATH; end".to_string()),
            "powershell" | "pwsh" => Some("$asdfDataDir = if ($env:ASDF_DATA_DIR) { $env:ASDF_DATA_DIR } else { \"$HOME/.asdf\" }; $sep = [IO.Path]::PathSeparator; $env:PATH = \"$asdfDataDir/bin$sep$asdfDataDir/shims$sep$env:PATH\"".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use versi_backend::{ShellInitOptions, VersionManager};

    use super::AsdfBackend;

    fn backend() -> AsdfBackend {
        AsdfBackend::new(PathBuf::from("asdf"), Some("0.18.0".to_string()), None)
    }

    #[test]
    fn capabilities_match_asdf_support() {
        let caps = backend().capabilities();

        assert!(!caps.supports_lts_filter);
        assert!(!caps.supports_use_version);
        assert!(caps.supports_shell_integration);
        assert!(!caps.supports_auto_switch);
        assert!(!caps.supports_corepack);
        assert!(!caps.supports_resolve_engines);
        assert!(caps.supports_uninstall);
    }

    #[test]
    fn shell_init_command_supports_expected_shells() {
        let options = ShellInitOptions::default();

        assert!(backend().shell_init_command("bash", &options).is_some());
        assert!(backend().shell_init_command("zsh", &options).is_some());
        assert!(backend().shell_init_command("fish", &options).is_some());
        assert!(backend().shell_init_command("pwsh", &options).is_some());
        assert!(backend().shell_init_command("nu", &options).is_none());
    }
}
