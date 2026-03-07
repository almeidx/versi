use crate::config::ShellConfig;
use crate::detect::ShellType;
use std::path::PathBuf;
use thiserror::Error;
use tokio::process::Command;
use versi_backend::ShellInitOptions;
use versi_platform::HideWindow;

#[cfg(target_os = "windows")]
use std::process::Stdio;
#[cfg(target_os = "windows")]
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub enum VerificationResult {
    Configured(Option<ShellInitOptions>),
    NotConfigured,
    ConfigFileNotFound,
    FunctionalButNotInConfig,
    Error(VerificationError),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VerificationError {
    #[error("failed to load shell config: {0}")]
    ConfigLoad(ShellConfigLoadError),
    #[error(transparent)]
    Wsl(#[from] WslShellConfigError),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ShellConfigLoadError {
    #[error("Config file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("IO error ({kind}): {details}")]
    Io {
        kind: std::io::ErrorKind,
        details: String,
    },
    #[error("Shell type does not support config files")]
    UnsupportedShell,
}

impl From<crate::config::ConfigError> for ShellConfigLoadError {
    fn from(value: crate::config::ConfigError) -> Self {
        match value {
            crate::config::ConfigError::FileNotFound(path) => Self::FileNotFound(path),
            crate::config::ConfigError::IoError(error) => Self::Io {
                kind: error.kind(),
                details: error.to_string(),
            },
            crate::config::ConfigError::UnsupportedShell => Self::UnsupportedShell,
        }
    }
}

impl From<crate::config::ConfigError> for VerificationError {
    fn from(value: crate::config::ConfigError) -> Self {
        Self::ConfigLoad(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WslShellConfigError {
    #[error("Shell not supported in WSL")]
    UnsupportedShell,
    #[error("{context}: {details}")]
    CommandFailure {
        context: &'static str,
        details: String,
    },
    #[error("WSL is only available on Windows")]
    UnsupportedPlatform,
}

impl WslShellConfigError {
    #[cfg(target_os = "windows")]
    fn command(context: &'static str, details: impl Into<String>) -> Self {
        Self::CommandFailure {
            context,
            details: details.into(),
        }
    }
}

pub async fn verify_shell_config(
    shell_type: ShellType,
    marker: &str,
    backend_binary: &str,
) -> VerificationResult {
    let config_files = shell_type.config_files();
    let existing_config = config_files.iter().find(|p| p.exists());

    match existing_config {
        Some(config_path) => match ShellConfig::load(shell_type, config_path.clone()) {
            Ok(config) => {
                if config.has_init(marker) {
                    let options = config.detect_options(marker);
                    VerificationResult::Configured(options)
                } else if functional_test(shell_type, backend_binary).await {
                    VerificationResult::FunctionalButNotInConfig
                } else {
                    VerificationResult::NotConfigured
                }
            }
            Err(error) => VerificationResult::Error(VerificationError::from(error)),
        },
        None => VerificationResult::ConfigFileNotFound,
    }
}

async fn functional_test(shell_type: ShellType, backend_binary: &str) -> bool {
    let version_cmd = format!("{backend_binary} --version");
    match shell_type {
        ShellType::Bash => Command::new("bash")
            .args(["-i", "-c", &version_cmd])
            .hide_window()
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false),
        ShellType::Zsh => Command::new("zsh")
            .args(["-i", "-c", &version_cmd])
            .hide_window()
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false),
        ShellType::Fish => Command::new("fish")
            .args(["-c", &version_cmd])
            .hide_window()
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false),
        ShellType::PowerShell => {
            let shell = if which::which("pwsh").is_ok() {
                "pwsh"
            } else {
                "powershell"
            };
            Command::new(shell)
                .args(["-Command", &version_cmd])
                .hide_window()
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false)
        }
        ShellType::Cmd => false,
    }
}

fn get_config_path_for_shell(shell_type: ShellType) -> Option<PathBuf> {
    shell_type.config_files().into_iter().find(|p| p.exists())
}

#[must_use]
pub fn get_or_create_config_path(shell_type: ShellType) -> Option<PathBuf> {
    if let Some(existing) = get_config_path_for_shell(shell_type) {
        return Some(existing);
    }

    shell_type.config_files().into_iter().next()
}

#[cfg(target_os = "windows")]
pub async fn verify_wsl_shell_config(
    shell_type: ShellType,
    distro: &str,
    marker: &str,
    backend_binary: &str,
) -> VerificationResult {
    use log::{debug, warn};

    let config_path = match shell_type {
        ShellType::Bash => "~/.bashrc",
        ShellType::Zsh => "~/.zshrc",
        ShellType::Fish => "~/.config/fish/config.fish",
        _ => {
            return VerificationResult::Error(VerificationError::Wsl(
                WslShellConfigError::UnsupportedShell,
            ));
        }
    };

    debug!(
        "Verifying {} config in WSL distro {}: {}",
        shell_type.name(),
        distro,
        config_path
    );

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", "cat", config_path])
        .hide_window()
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout);
                if content.contains(marker) {
                    let marker_lines: String = content
                        .lines()
                        .filter(|line| line.contains(marker))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let options = ShellInitOptions {
                        use_on_cd: marker_lines.contains("--use-on-cd"),
                        resolve_engines: marker_lines.contains("--resolve-engines"),
                        corepack_enabled: marker_lines.contains("--corepack-enabled"),
                    };
                    debug!("WSL shell {} is configured", shell_type.name());
                    VerificationResult::Configured(Some(options))
                } else if wsl_functional_test(shell_type, distro, backend_binary).await {
                    debug!(
                        "WSL shell {} is functional but not in config",
                        shell_type.name()
                    );
                    VerificationResult::FunctionalButNotInConfig
                } else {
                    debug!("WSL shell {} is not configured", shell_type.name());
                    VerificationResult::NotConfigured
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("No such file") || stderr.contains("cannot access") {
                    debug!("WSL config file not found: {}", config_path);
                    VerificationResult::ConfigFileNotFound
                } else {
                    warn!("WSL cat failed: {}", stderr);
                    VerificationResult::Error(VerificationError::Wsl(WslShellConfigError::command(
                        "Failed to read WSL config",
                        stderr.trim(),
                    )))
                }
            }
        }
        Err(error) => {
            warn!("Failed to read WSL config: {}", error);
            VerificationResult::Error(VerificationError::Wsl(WslShellConfigError::command(
                "Failed to read WSL config",
                error.to_string(),
            )))
        }
    }
}

#[cfg(target_os = "windows")]
async fn wsl_functional_test(shell_type: ShellType, distro: &str, backend_binary: &str) -> bool {
    use log::debug;

    let version_cmd = format!("{} --version", backend_binary);
    let (shell_cmd, args) = match shell_type {
        ShellType::Bash => ("bash", vec!["-i", "-c", &version_cmd]),
        ShellType::Zsh => ("zsh", vec!["-i", "-c", &version_cmd]),
        ShellType::Fish => ("fish", vec!["-c", &version_cmd]),
        _ => return false,
    };

    debug!(
        "Running WSL functional test for {} in {}",
        shell_type.name(),
        distro
    );

    let mut cmd_args = vec!["-d", distro, "--", shell_cmd];
    cmd_args.extend(args);

    Command::new("wsl.exe")
        .args(&cmd_args)
        .hide_window()
        .output()
        .await
        .map(|o| {
            debug!("WSL functional test result: {}", o.status.success());
            o.status.success()
        })
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
/// Configure shell initialization inside a WSL distribution.
///
/// # Errors
/// Returns an error if the target shell is unsupported, or if reading/updating
/// the remote config file fails.
pub async fn configure_wsl_shell_config(
    shell_type: ShellType,
    distro: &str,
    marker: &str,
    label: &str,
    init_command: &str,
    options: &ShellInitOptions,
) -> Result<(), WslShellConfigError> {
    let config_path = wsl_config_path(shell_type)?;
    let content = read_wsl_config_file(distro, config_path).await?;
    let mut config = ShellConfig {
        shell_type,
        config_path: PathBuf::from(config_path),
        content,
    };

    let edit = if config.has_init(marker) {
        config.update_flags(marker, options)
    } else {
        config.add_init(init_command, label)
    };

    if edit.has_changes() {
        write_wsl_config_file(distro, config_path, &edit.modified).await?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn wsl_config_path(shell_type: ShellType) -> Result<&'static str, WslShellConfigError> {
    match shell_type {
        ShellType::Bash => Ok("$HOME/.bashrc"),
        ShellType::Zsh => Ok("$HOME/.zshrc"),
        ShellType::Fish => Ok("$HOME/.config/fish/config.fish"),
        _ => Err(WslShellConfigError::UnsupportedShell),
    }
}

#[cfg(target_os = "windows")]
async fn read_wsl_config_file(
    distro: &str,
    config_path: &str,
) -> Result<String, WslShellConfigError> {
    let output = Command::new("wsl.exe")
        .args([
            "-d",
            distro,
            "--",
            "sh",
            "-c",
            "config=\"$1\"; if [ -f \"$config\" ]; then cat \"$config\"; fi",
            "sh",
            config_path,
        ])
        .hide_window()
        .output()
        .await
        .map_err(|error| {
            WslShellConfigError::command("Failed to read WSL config", error.to_string())
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(WslShellConfigError::command(
            "Failed to read WSL config",
            if stderr.is_empty() {
                "command exited unsuccessfully".to_string()
            } else {
                stderr
            },
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(target_os = "windows")]
async fn write_wsl_config_file(
    distro: &str,
    config_path: &str,
    content: &str,
) -> Result<(), WslShellConfigError> {
    let mut child = Command::new("wsl.exe")
        .args([
            "-d",
            distro,
            "--",
            "sh",
            "-c",
            "config=\"$1\"; mkdir -p \"$(dirname \"$config\")\"; cat > \"$config\"",
            "sh",
            config_path,
        ])
        .hide_window()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            WslShellConfigError::command("Failed to open WSL config for writing", error.to_string())
        })?;

    let Some(mut stdin) = child.stdin.take() else {
        return Err(WslShellConfigError::command(
            "Failed to write WSL config",
            "stdin unavailable",
        ));
    };
    stdin.write_all(content.as_bytes()).await.map_err(|error| {
        WslShellConfigError::command("Failed to write WSL config content", error.to_string())
    })?;
    drop(stdin);

    let output = child.wait_with_output().await.map_err(|error| {
        WslShellConfigError::command("Failed to finalize WSL config write", error.to_string())
    })?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(WslShellConfigError::command(
            "Failed to write WSL config",
            if stderr.is_empty() {
                "command exited unsuccessfully".to_string()
            } else {
                stderr
            },
        ))
    }
}

#[cfg(not(target_os = "windows"))]
pub async fn verify_wsl_shell_config(
    _shell_type: ShellType,
    _distro: &str,
    _marker: &str,
    _backend_binary: &str,
) -> VerificationResult {
    VerificationResult::Error(VerificationError::Wsl(
        WslShellConfigError::UnsupportedPlatform,
    ))
}

#[cfg(not(target_os = "windows"))]
/// Configure shell initialization inside a WSL distribution.
///
/// # Errors
/// Always returns an error on non-Windows platforms because WSL is unavailable.
pub async fn configure_wsl_shell_config(
    _shell_type: ShellType,
    _distro: &str,
    _marker: &str,
    _label: &str,
    _init_command: &str,
    _options: &ShellInitOptions,
) -> Result<(), WslShellConfigError> {
    Err(WslShellConfigError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use crate::detect::ShellType;

    #[cfg(not(target_os = "windows"))]
    use super::{
        VerificationError, WslShellConfigError, configure_wsl_shell_config, verify_wsl_shell_config,
    };
    use super::{get_config_path_for_shell, get_or_create_config_path};
    #[cfg(not(target_os = "windows"))]
    use versi_backend::ShellInitOptions;

    #[test]
    fn cmd_shell_has_no_config_path() {
        assert!(get_config_path_for_shell(ShellType::Cmd).is_none());
        assert!(get_or_create_config_path(ShellType::Cmd).is_none());
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn wsl_verify_returns_platform_error_on_non_windows() {
        let result = verify_wsl_shell_config(ShellType::Bash, "Ubuntu", "fnm env", "fnm").await;

        assert!(matches!(
            result,
            super::VerificationResult::Error(VerificationError::Wsl(
                WslShellConfigError::UnsupportedPlatform
            ))
        ));
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn wsl_configure_returns_platform_error_on_non_windows() {
        let result = configure_wsl_shell_config(
            ShellType::Bash,
            "Ubuntu",
            "fnm env",
            "fnm",
            "eval \"$(fnm env)\"",
            &ShellInitOptions::default(),
        )
        .await;

        assert_eq!(result, Err(WslShellConfigError::UnsupportedPlatform));
    }
}
