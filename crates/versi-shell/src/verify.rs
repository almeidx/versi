use crate::config::ShellConfig;
use crate::detect::{FnmShellOptions, ShellType};
use std::path::PathBuf;
use tokio::process::Command;

#[cfg(windows)]
use tokio::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

trait HideWindow {
    fn hide_window(&mut self) -> &mut Self;
}

impl HideWindow for Command {
    #[cfg(windows)]
    fn hide_window(&mut self) -> &mut Self {
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(windows))]
    fn hide_window(&mut self) -> &mut Self {
        self
    }
}

#[derive(Debug, Clone)]
pub enum VerificationResult {
    Configured(Option<FnmShellOptions>),
    NotConfigured,
    ConfigFileNotFound,
    FunctionalButNotInConfig,
    Error(String),
}

pub async fn verify_shell_config(shell_type: &ShellType) -> VerificationResult {
    let config_files = shell_type.config_files();
    let existing_config = config_files.iter().find(|p| p.exists());

    match existing_config {
        Some(config_path) => match ShellConfig::load(shell_type.clone(), config_path.clone()) {
            Ok(config) => {
                if config.has_fnm_init() {
                    let options = config.detect_fnm_options();
                    VerificationResult::Configured(options)
                } else if functional_test(shell_type).await {
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

async fn functional_test(shell_type: &ShellType) -> bool {
    match shell_type {
        ShellType::Bash => Command::new("bash")
            .args(["-i", "-c", "fnm --version"])
            .hide_window()
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false),
        ShellType::Zsh => Command::new("zsh")
            .args(["-i", "-c", "fnm --version"])
            .hide_window()
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false),
        ShellType::Fish => Command::new("fish")
            .args(["-c", "fnm --version"])
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
                .args(["-Command", "fnm --version"])
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
