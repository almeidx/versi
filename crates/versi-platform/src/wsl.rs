use std::time::Duration;

use log::{debug, error, info, trace, warn};
use thiserror::Error;
use tokio::process::Command;

use crate::HideWindow;

#[derive(Debug, Clone, Copy)]
pub struct WslTimeouts {
    pub list: Duration,
    pub distro: Duration,
}

#[derive(Debug, Clone)]
pub struct WslDistro {
    pub name: String,
    pub is_default: bool,
    pub version: u8,
    pub backend_path: Option<String>,
    pub is_running: bool,
}

#[derive(Error, Debug)]
pub enum WslError {
    #[error("WSL not available")]
    NotAvailable,

    #[error("Command failed: {stderr}")]
    CommandFailed { stderr: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub async fn detect_wsl_distros(search_paths: &[&str], timeouts: &WslTimeouts) -> Vec<WslDistro> {
    info!("Detecting WSL distros...");

    let running_distros = get_running_distro_names(timeouts.list).await;
    debug!("Running distros: {:?}", running_distros);

    debug!("Running: wsl.exe --list --verbose");
    let output = tokio::time::timeout(
        timeouts.list,
        Command::new("wsl.exe")
            .args(["--list", "--verbose"])
            .hide_window()
            .output(),
    )
    .await;

    let output = match output {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            error!("Failed to execute wsl.exe: {}", e);
            return Vec::new();
        }
        Err(_) => {
            warn!(
                "wsl.exe --list --verbose timed out after {:?}",
                timeouts.list
            );
            return Vec::new();
        }
    };

    debug!("wsl.exe exit status: {:?}", output.status);
    trace!("wsl.exe stdout raw bytes: {:?}", &output.stdout);
    trace!(
        "wsl.exe stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    if !output.status.success() {
        warn!(
            "wsl.exe command failed with status: {:?}, stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        return Vec::new();
    }

    let stdout = decode_wsl_output(&output.stdout);
    debug!("Decoded WSL output:\n{}", stdout);

    let mut distros = parse_wsl_list(&stdout, &running_distros);
    info!("Found {} WSL distros", distros.len());

    for distro in &mut distros {
        if distro.is_running {
            debug!("Checking for backend in running distro: {}", distro.name);
            distro.backend_path =
                find_backend_path(&distro.name, search_paths, timeouts.distro).await;
            if let Some(ref path) = distro.backend_path {
                info!("Found backend in {}: {}", distro.name, path);
            } else {
                warn!("Backend not found in distro: {}", distro.name);
            }
        } else {
            debug!(
                "Skipping backend check for non-running distro: {}",
                distro.name
            );
        }
    }

    let (with_backend, running) = distros.iter().fold((0usize, 0usize), |(b, r), d| {
        (
            b + usize::from(d.backend_path.is_some()),
            r + usize::from(d.is_running),
        )
    });
    info!(
        "WSL detection complete: {} distros with backend, {} running, {} total",
        with_backend,
        running,
        distros.len()
    );
    distros
}

async fn get_running_distro_names(timeout: Duration) -> Vec<String> {
    let output = tokio::time::timeout(
        timeout,
        Command::new("wsl.exe")
            .args(["--list", "--running", "--quiet"])
            .hide_window()
            .output(),
    )
    .await;

    match output {
        Ok(Ok(output)) if output.status.success() => {
            let stdout = decode_wsl_output(&output.stdout);
            stdout
                .lines()
                .map(|line| line.trim().replace('\0', ""))
                .filter(|line| !line.is_empty())
                .collect()
        }
        Err(_) => {
            warn!("wsl.exe --list --running timed out after {:?}", timeout);
            Vec::new()
        }
        _ => Vec::new(),
    }
}

async fn find_backend_path(
    distro: &str,
    search_paths: &[&str],
    timeout: Duration,
) -> Option<String> {
    if search_paths.is_empty() {
        return None;
    }

    let check_cmd = search_paths
        .iter()
        .map(|p| format!("[ -x {} ] && {{ echo {}; exit 0; }}", p, p))
        .collect::<Vec<_>>()
        .join("; ");

    debug!(
        "Running backend path detection for {}: wsl.exe -d {} -- sh -c \"{}\"",
        distro, distro, check_cmd
    );

    let output = tokio::time::timeout(
        timeout,
        Command::new("wsl.exe")
            .args(["-d", distro, "--", "sh", "-c", &check_cmd])
            .hide_window()
            .output(),
    )
    .await;

    match output {
        Ok(Ok(output)) => {
            debug!(
                "Backend path detection for {} - exit status: {:?}",
                distro, output.status
            );
            trace!(
                "Backend path detection stdout: {:?}",
                String::from_utf8_lossy(&output.stdout)
            );
            trace!(
                "Backend path detection stderr: {:?}",
                String::from_utf8_lossy(&output.stderr)
            );

            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                if let Some(ref p) = path {
                    debug!("Backend found at: {}", p);
                    return path;
                }
                debug!("Backend path detection returned empty output");
            } else {
                warn!(
                    "Backend path detection failed for {}: {}",
                    distro,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Ok(Err(e)) => {
            error!("Failed to run backend path detection for {}: {}", distro, e);
        }
        Err(_) => {
            warn!(
                "Backend path detection for {} timed out after {:?}",
                distro, timeout
            );
        }
    }

    None
}

fn decode_wsl_output(bytes: &[u8]) -> String {
    let looks_utf16le = bytes.len() >= 2
        && bytes.len() % 2 == 0
        && bytes.iter().skip(1).step_by(2).any(|&b| b == 0);

    if looks_utf16le {
        let u16_iter = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
        let decoded: String = char::decode_utf16(u16_iter)
            .filter_map(|r| r.ok())
            .collect();
        if !decoded.is_empty() {
            return decoded;
        }
    }
    String::from_utf8_lossy(bytes).to_string()
}

fn parse_wsl_list(output: &str, running_distros: &[String]) -> Vec<WslDistro> {
    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let line = line.trim().replace('\0', "");
            if line.is_empty() {
                return None;
            }

            let is_default = line.starts_with('*');
            let line = line.trim_start_matches('*').trim();

            let (name, version_token) = split_wsl_row(line);
            if name.is_empty() {
                return None;
            }
            let is_running = running_distros.iter().any(|r| r == name);
            let version = match version_token {
                Some(v) => v.parse().unwrap_or_else(|e| {
                    debug!("Failed to parse WSL version for {name:?}: {e}, defaulting to 2");
                    2
                }),
                None => 2,
            };
            Some(WslDistro {
                name: name.to_string(),
                is_default,
                version,
                backend_path: None,
                is_running,
            })
        })
        .collect()
}

fn split_wsl_row(line: &str) -> (&str, Option<&str>) {
    let bytes = line.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return ("", None);
    }

    let mut version_start = end;
    while version_start > 0 && !bytes[version_start - 1].is_ascii_whitespace() {
        version_start -= 1;
    }
    let version = &line[version_start..end];

    let mut state_end = version_start;
    while state_end > 0 && bytes[state_end - 1].is_ascii_whitespace() {
        state_end -= 1;
    }
    if state_end == 0 {
        return (line.trim(), None);
    }

    let mut state_start = state_end;
    while state_start > 0 && !bytes[state_start - 1].is_ascii_whitespace() {
        state_start -= 1;
    }
    let name = line[..state_start].trim();

    if name.is_empty() {
        (line.trim(), None)
    } else {
        (name, Some(version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_wsl_output_utf8() {
        let input = b"Ubuntu Running 2";
        let result = decode_wsl_output(input);
        assert!(result.contains("Ubuntu"));
    }

    #[test]
    fn test_decode_wsl_output_utf16le() {
        let input: Vec<u8> = "Ubuntu"
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let result = decode_wsl_output(&input);
        assert!(result.contains("Ubuntu"));
    }

    #[test]
    fn test_parse_wsl_list_basic() {
        let output = "  NAME      STATE           VERSION\n* Ubuntu    Running         2\n  Debian    Stopped         2";
        let running = vec!["Ubuntu".to_string()];
        let distros = parse_wsl_list(output, &running);

        assert_eq!(distros.len(), 2);
        assert_eq!(distros[0].name, "Ubuntu");
        assert!(distros[0].is_default);
        assert!(distros[0].is_running);
        assert_eq!(distros[0].version, 2);

        assert_eq!(distros[1].name, "Debian");
        assert!(!distros[1].is_default);
        assert!(!distros[1].is_running);
    }

    #[test]
    fn test_parse_wsl_list_empty() {
        let output = "  NAME      STATE           VERSION\n";
        let running: Vec<String> = vec![];
        let distros = parse_wsl_list(output, &running);
        assert!(distros.is_empty());
    }

    #[test]
    fn test_parse_wsl_list_skips_header() {
        let output = "  NAME      STATE           VERSION\nUbuntu    Running         2";
        let running = vec!["Ubuntu".to_string()];
        let distros = parse_wsl_list(output, &running);

        assert_eq!(distros.len(), 1);
        assert_eq!(distros[0].name, "Ubuntu");
    }

    #[test]
    fn test_parse_wsl_list_version_parsing() {
        let output = "  NAME      STATE           VERSION\nUbuntu    Running         1";
        let running = vec!["Ubuntu".to_string()];
        let distros = parse_wsl_list(output, &running);

        assert_eq!(distros[0].version, 1);
    }

    #[test]
    fn test_parse_wsl_list_default_marker() {
        let output = "  NAME      STATE           VERSION\n* Ubuntu    Running         2\n  Debian    Stopped         2";
        let running: Vec<String> = vec![];
        let distros = parse_wsl_list(output, &running);

        assert!(distros[0].is_default);
        assert!(!distros[1].is_default);
    }

    #[test]
    fn test_parse_wsl_list_running_detection() {
        let output = "  NAME      STATE           VERSION\nUbuntu    Running         2\nDebian    Stopped         2";
        let running = vec!["Ubuntu".to_string()];
        let distros = parse_wsl_list(output, &running);

        assert!(distros[0].is_running);
        assert!(!distros[1].is_running);
    }

    #[test]
    fn test_parse_wsl_list_with_null_chars() {
        let output = "  NAME      STATE           VERSION\nUbuntu\0    Running         2";
        let running = vec!["Ubuntu".to_string()];
        let distros = parse_wsl_list(output, &running);

        assert_eq!(distros[0].name, "Ubuntu");
    }

    #[test]
    fn test_parse_wsl_list_minimal_format() {
        let output = "  NAME      STATE           VERSION\nUbuntu";
        let running: Vec<String> = vec![];
        let distros = parse_wsl_list(output, &running);

        assert_eq!(distros.len(), 1);
        assert_eq!(distros[0].name, "Ubuntu");
        assert_eq!(distros[0].version, 2);
    }

    #[test]
    fn test_parse_wsl_list_name_with_spaces() {
        let output = "  NAME      STATE           VERSION\n* Ubuntu 22.04    Running         2\n  Debian    Stopped         2";
        let running = vec!["Ubuntu 22.04".to_string()];
        let distros = parse_wsl_list(output, &running);

        assert_eq!(distros.len(), 2);
        assert_eq!(distros[0].name, "Ubuntu 22.04");
        assert!(distros[0].is_running);
        assert_eq!(distros[0].version, 2);
    }

    #[test]
    fn test_wsl_distro_backend_path_default() {
        let output = "  NAME      STATE           VERSION\nUbuntu    Running         2";
        let running: Vec<String> = vec![];
        let distros = parse_wsl_list(output, &running);

        assert!(distros[0].backend_path.is_none());
    }
}

pub async fn execute_in_wsl(distro: &str, command: &str) -> Result<String, WslError> {
    debug!(
        "Executing in WSL {}: wsl.exe -d {} -- sh -c \"{}\"",
        distro, distro, command
    );

    let output = tokio::process::Command::new("wsl.exe")
        .args(["-d", distro, "--", "sh", "-c", command])
        .hide_window()
        .output()
        .await?;

    debug!("WSL command exit status: {:?}", output.status);
    trace!(
        "WSL command stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    trace!(
        "WSL command stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        debug!(
            "WSL command succeeded, output length: {} bytes",
            stdout.len()
        );
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!(
            "WSL command failed in {}: command='{}', stderr='{}'",
            distro, command, stderr
        );
        Err(WslError::CommandFailed { stderr })
    }
}
