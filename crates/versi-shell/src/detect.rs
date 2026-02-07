use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use which::which;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Cmd,
}

impl ShellType {
    pub fn name(&self) -> &'static str {
        match self {
            ShellType::Bash => "Bash",
            ShellType::Zsh => "Zsh",
            ShellType::Fish => "Fish",
            ShellType::PowerShell => "PowerShell",
            ShellType::Cmd => "Command Prompt",
        }
    }

    pub fn shell_arg(&self) -> &'static str {
        match self {
            ShellType::Bash => "bash",
            ShellType::Zsh => "zsh",
            ShellType::Fish => "fish",
            ShellType::PowerShell => "powershell",
            ShellType::Cmd => "cmd",
        }
    }

    pub fn config_files(&self) -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();

        match self {
            ShellType::Bash => vec![
                home.join(".bashrc"),
                home.join(".bash_profile"),
                home.join(".profile"),
            ],
            ShellType::Zsh => vec![home.join(".zshrc"), home.join(".zprofile")],
            ShellType::Fish => vec![home.join(".config/fish/config.fish")],
            ShellType::PowerShell => {
                #[cfg(target_os = "windows")]
                {
                    if let Some(docs) = dirs::document_dir() {
                        vec![
                            docs.join("PowerShell/Microsoft.PowerShell_profile.ps1"),
                            docs.join("WindowsPowerShell/Microsoft.PowerShell_profile.ps1"),
                        ]
                    } else {
                        vec![]
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    vec![home.join(".config/powershell/Microsoft.PowerShell_profile.ps1")]
                }
            }
            ShellType::Cmd => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShellInfo {
    pub shell_type: ShellType,
    pub path: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
    pub is_configured: bool,
}

pub fn detect_shells() -> Vec<ShellInfo> {
    detect_native_shells()
}

pub fn detect_native_shells() -> Vec<ShellInfo> {
    let mut shells = Vec::new();

    #[cfg(unix)]
    {
        if let Ok(path) = which("bash") {
            let config_file = find_existing_config(&ShellType::Bash);
            shells.push(ShellInfo {
                shell_type: ShellType::Bash,
                path: Some(path),
                config_file,
                is_configured: false,
            });
        }

        if let Ok(path) = which("zsh") {
            let config_file = find_existing_config(&ShellType::Zsh);
            shells.push(ShellInfo {
                shell_type: ShellType::Zsh,
                path: Some(path),
                config_file,
                is_configured: false,
            });
        }

        if let Ok(path) = which("fish") {
            let config_file = find_existing_config(&ShellType::Fish);
            shells.push(ShellInfo {
                shell_type: ShellType::Fish,
                path: Some(path),
                config_file,
                is_configured: false,
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        let powershell_path = which("pwsh")
            .ok()
            .or_else(|| which("powershell").ok())
            .or_else(|| {
                let system_root =
                    std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
                let legacy_path = PathBuf::from(&system_root)
                    .join("System32\\WindowsPowerShell\\v1.0\\powershell.exe");
                if legacy_path.exists() {
                    Some(legacy_path)
                } else {
                    None
                }
            });

        if powershell_path.is_some() {
            let config_file = find_existing_config(&ShellType::PowerShell);
            shells.push(ShellInfo {
                shell_type: ShellType::PowerShell,
                path: powershell_path,
                config_file,
                is_configured: false,
            });
        }

        shells.push(ShellInfo {
            shell_type: ShellType::Cmd,
            path: Some(PathBuf::from("cmd.exe")),
            config_file: None,
            is_configured: false,
        });
    }

    shells
}

#[cfg(target_os = "windows")]
pub fn detect_wsl_shells(distro: &str) -> Vec<ShellInfo> {
    use log::{debug, warn};
    use std::process::Command;
    use versi_platform::HideWindow;

    let mut shells = Vec::new();

    let check_shells_script = r#"
        for shell in bash zsh fish; do
            if command -v "$shell" >/dev/null 2>&1; then
                echo "$shell:$(command -v "$shell")"
            fi
        done
    "#;

    debug!("Detecting shells in WSL distro: {}", distro);

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", "sh", "-c", check_shells_script])
        .hide_window()
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                debug!("WSL shell detection output: {}", stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 {
                        let shell_name = parts[0].trim();
                        let shell_path = parts[1].trim();

                        let shell_type = match shell_name {
                            "bash" => ShellType::Bash,
                            "zsh" => ShellType::Zsh,
                            "fish" => ShellType::Fish,
                            _ => continue,
                        };

                        shells.push(ShellInfo {
                            shell_type,
                            path: Some(PathBuf::from(shell_path)),
                            config_file: None,
                            is_configured: false,
                        });
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    "WSL shell detection failed for {}: status={:?}, stderr={}",
                    distro, output.status, stderr
                );
            }
        }
        Err(e) => {
            warn!("Failed to run WSL shell detection for {}: {}", distro, e);
        }
    }

    debug!("Detected {} shells in WSL distro {}", shells.len(), distro);
    shells
}

#[cfg(not(target_os = "windows"))]
pub fn detect_wsl_shells(_distro: &str) -> Vec<ShellInfo> {
    Vec::new()
}

fn find_existing_config(shell: &ShellType) -> Option<PathBuf> {
    shell.config_files().into_iter().find(|path| path.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_type_name() {
        assert_eq!(ShellType::Bash.name(), "Bash");
        assert_eq!(ShellType::Zsh.name(), "Zsh");
        assert_eq!(ShellType::Fish.name(), "Fish");
        assert_eq!(ShellType::PowerShell.name(), "PowerShell");
        assert_eq!(ShellType::Cmd.name(), "Command Prompt");
    }

    #[test]
    fn test_shell_type_shell_arg() {
        assert_eq!(ShellType::Bash.shell_arg(), "bash");
        assert_eq!(ShellType::Zsh.shell_arg(), "zsh");
        assert_eq!(ShellType::Fish.shell_arg(), "fish");
        assert_eq!(ShellType::PowerShell.shell_arg(), "powershell");
        assert_eq!(ShellType::Cmd.shell_arg(), "cmd");
    }

    #[test]
    fn test_config_files_bash() {
        let files = ShellType::Bash.config_files();
        assert!(!files.is_empty());
        assert!(files.iter().any(|p| p.ends_with(".bashrc")));
    }

    #[test]
    fn test_config_files_zsh() {
        let files = ShellType::Zsh.config_files();
        assert!(!files.is_empty());
        assert!(files.iter().any(|p| p.ends_with(".zshrc")));
    }

    #[test]
    fn test_config_files_fish() {
        let files = ShellType::Fish.config_files();
        assert!(!files.is_empty());
        assert!(files.iter().any(|p| p.to_string_lossy().contains("fish")));
    }

    #[test]
    fn test_config_files_cmd() {
        let files = ShellType::Cmd.config_files();
        assert!(files.is_empty());
    }

    #[test]
    fn test_shell_type_equality() {
        assert_eq!(ShellType::Bash, ShellType::Bash);
        assert_ne!(ShellType::Bash, ShellType::Zsh);
    }

    #[test]
    fn test_shell_type_clone() {
        let shell = ShellType::Bash;
        let cloned = shell.clone();
        assert_eq!(shell, cloned);
    }
}
