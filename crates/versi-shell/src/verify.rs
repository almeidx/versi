use crate::config::ShellConfig;
use crate::detect::ShellType;
use std::path::PathBuf;
use tokio::process::Command;
use versi_backend::ShellInitOptions;
use versi_platform::HideWindow;

#[derive(Debug, Clone)]
pub enum VerificationResult {
    Configured(Option<ShellInitOptions>),
    NotConfigured,
    ConfigFileNotFound,
    FunctionalButNotInConfig,
    Error(String),
}

pub async fn verify_shell_config(
    shell_type: &ShellType,
    marker: &str,
    backend_binary: &str,
) -> VerificationResult {
    let config_files = shell_type.config_files();
    let existing_config = config_files.iter().find(|p| p.exists());

    match existing_config {
        Some(config_path) => match ShellConfig::load(shell_type.clone(), config_path.clone()) {
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
            Err(e) => VerificationResult::Error(e.to_string()),
        },
        None => VerificationResult::ConfigFileNotFound,
    }
}

async fn functional_test(shell_type: &ShellType, backend_binary: &str) -> bool {
    let version_cmd = format!("{} --version", backend_binary);
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

pub fn get_config_path_for_shell(shell_type: &ShellType) -> Option<PathBuf> {
    shell_type.config_files().into_iter().find(|p| p.exists())
}

pub fn get_or_create_config_path(shell_type: &ShellType) -> Option<PathBuf> {
    if let Some(existing) = get_config_path_for_shell(shell_type) {
        return Some(existing);
    }

    shell_type.config_files().into_iter().next()
}

#[cfg(target_os = "windows")]
pub async fn verify_wsl_shell_config(
    shell_type: &ShellType,
    distro: &str,
    marker: &str,
    backend_binary: &str,
) -> VerificationResult {
    use log::{debug, warn};

    let config_path = match shell_type {
        ShellType::Bash => "~/.bashrc",
        ShellType::Zsh => "~/.zshrc",
        ShellType::Fish => "~/.config/fish/config.fish",
        _ => return VerificationResult::Error("Shell not supported in WSL".to_string()),
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
                    let options = ShellInitOptions {
                        use_on_cd: content.contains("--use-on-cd"),
                        resolve_engines: content.contains("--resolve-engines"),
                        corepack_enabled: content.contains("--corepack-enabled"),
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
                    VerificationResult::Error(stderr.to_string())
                }
            }
        }
        Err(e) => {
            warn!("Failed to read WSL config: {}", e);
            VerificationResult::Error(e.to_string())
        }
    }
}

#[cfg(target_os = "windows")]
async fn wsl_functional_test(shell_type: &ShellType, distro: &str, backend_binary: &str) -> bool {
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

#[cfg(not(target_os = "windows"))]
pub async fn verify_wsl_shell_config(
    _shell_type: &ShellType,
    _distro: &str,
    _marker: &str,
    _backend_binary: &str,
) -> VerificationResult {
    VerificationResult::Error("WSL is only available on Windows".to_string())
}
