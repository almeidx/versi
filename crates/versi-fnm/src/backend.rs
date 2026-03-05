use async_trait::async_trait;
use log::{debug, error, info, trace};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;

use versi_core::HideWindow;

use versi_backend::{
    BackendError, BackendInfo, CommandEnvironment, InstallProgress, InstalledVersion,
    ManagerCapabilities, NodeVersion, RemoteVersion, ShellInitOptions, VersionManager,
    build_backend_command, execute_backend_command_with, find_default_version,
    parse_current_version, sanitize_terminal_text,
};

use crate::version::{parse_installed_versions, parse_remote_versions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProgressStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Default)]
struct InstallProgressParser {
    remainder: String,
    saw_download: bool,
    sent_extracting: bool,
    sent_configuring: bool,
    last_downloaded_bytes: Option<u64>,
    last_total_bytes: Option<u64>,
}

#[derive(Clone)]
pub struct FnmBackend {
    info: BackendInfo,
    command_env: CommandEnvironment,
}

impl FnmBackend {
    #[must_use]
    pub fn new(path: PathBuf, version: Option<String>, data_dir: Option<PathBuf>) -> Self {
        let command_env = CommandEnvironment::Native {
            binary_path: path.clone(),
        };
        Self {
            info: BackendInfo {
                name: "fnm",
                path,
                version,
                data_dir,
                in_path: true,
            },
            command_env,
        }
    }

    #[must_use]
    pub fn with_fnm_dir(mut self, dir: PathBuf) -> Self {
        self.info.data_dir = Some(dir);
        self
    }

    #[must_use]
    pub fn with_in_path(mut self, in_path: bool) -> Self {
        self.info.in_path = in_path;
        self
    }

    #[must_use]
    pub fn with_wsl(distro: String, fnm_path: String) -> Self {
        Self {
            info: BackendInfo {
                name: "fnm",
                path: PathBuf::from(&fnm_path),
                version: None,
                data_dir: None,
                in_path: false,
            },
            command_env: CommandEnvironment::Wsl {
                distro,
                binary_path: fnm_path,
            },
        }
    }

    fn apply_native_env(&self, cmd: &mut Command) {
        if !self.command_env.is_native() {
            return;
        }

        if let Some(dir) = &self.info.data_dir {
            debug!("Setting FNM_DIR={}", dir.display());
            cmd.env("FNM_DIR", dir);
        }
    }

    fn build_command(&self, args: &[&str]) -> Command {
        let mut cmd = build_backend_command(&self.command_env, args);
        self.apply_native_env(&mut cmd);
        cmd
    }

    fn should_use_tty_wrapper(&self) -> bool {
        if !self.command_env.is_native() {
            return false;
        }

        #[cfg(unix)]
        {
            which::which("script").is_ok()
        }

        #[cfg(not(unix))]
        {
            false
        }
    }

    fn build_script_install_command(&self, version: &str) -> Command {
        let mut cmd = Command::new("script");
        cmd.args(["-q", "/dev/null"]);
        if !cfg!(target_os = "macos") {
            cmd.arg("--");
        }
        cmd.arg(&self.info.path);
        cmd.args(["install", version, "--progress", "always"]);
        self.apply_native_env(&mut cmd);
        cmd.hide_window();
        cmd
    }

    fn build_install_command(&self, version: &str) -> (Command, ProgressStream) {
        if self.should_use_tty_wrapper() {
            info!("Executing fnm install with TTY progress wrapper");
            return (
                self.build_script_install_command(version),
                ProgressStream::Stdout,
            );
        }

        info!("Executing fnm install in direct mode (progress may be limited)");
        (
            self.build_command(&["install", version, "--progress", "always"]),
            ProgressStream::Stderr,
        )
    }

    async fn execute(&self, args: &[&str]) -> Result<String, BackendError> {
        execute_backend_command_with("fnm", &self.command_env, args, |cmd| {
            self.apply_native_env(cmd);
        })
        .await
    }

    async fn execute_install_with_progress(
        &self,
        version: &str,
        progress_tx: mpsc::Sender<InstallProgress>,
    ) -> Result<(), BackendError> {
        let (mut command, progress_stream) = self.build_install_command(version);
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = command.spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BackendError::BackendSpecific {
                context: "install spawn",
                details: "failed to capture stdout".to_string(),
            })?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| BackendError::BackendSpecific {
                context: "install spawn",
                details: "failed to capture stderr".to_string(),
            })?;

        let parse_stdout = progress_stream == ProgressStream::Stdout;
        let stdout_tx = progress_tx.clone();
        let stderr_tx = progress_tx.clone();

        let stdout_task =
            tokio::spawn(async move { read_stream(stdout, parse_stdout, stdout_tx).await });
        let stderr_task =
            tokio::spawn(async move { read_stream(stderr, !parse_stdout, stderr_tx).await });

        let status = child.wait().await?;

        let (stdout_bytes, mut stdout_parser) = stdout_task
            .await
            .map_err(|error| BackendError::BackendSpecific {
                context: "install stdout reader",
                details: format!("reader task failed: {error}"),
            })?
            .map_err(BackendError::from)?;
        let (stderr_bytes, mut stderr_parser) = stderr_task
            .await
            .map_err(|error| BackendError::BackendSpecific {
                context: "install stderr reader",
                details: format!("reader task failed: {error}"),
            })?
            .map_err(BackendError::from)?;

        debug!("fnm install exit status: {status:?}");
        trace!(
            "fnm install stdout: {}",
            String::from_utf8_lossy(&stdout_bytes)
        );
        if !stderr_bytes.is_empty() {
            trace!(
                "fnm install stderr: {}",
                String::from_utf8_lossy(&stderr_bytes)
            );
        }

        if status.success() {
            let parser = if parse_stdout {
                stdout_parser.as_mut()
            } else {
                stderr_parser.as_mut()
            };
            if let Some(parser) = parser {
                parser.finish(&progress_tx);
            }
            Ok(())
        } else {
            let stderr_text = String::from_utf8_lossy(&stderr_bytes);
            let stdout_text = String::from_utf8_lossy(&stdout_bytes);
            let details = combined_error_output(stderr_text.as_ref(), stdout_text.as_ref());
            error!("fnm install failed for {version}: {details}");
            Err(BackendError::CommandFailed { stderr: details })
        }
    }
}

impl InstallProgressParser {
    fn feed_bytes(&mut self, chunk: &[u8], progress_tx: &mpsc::Sender<InstallProgress>) {
        self.remainder.push_str(&String::from_utf8_lossy(chunk));

        while let Some(idx) = self.remainder.find(['\r', '\n']) {
            let line = self.remainder[..idx].to_string();
            self.remainder.drain(..=idx);
            self.process_line(&line, progress_tx);
        }
    }

    fn finish(&mut self, progress_tx: &mpsc::Sender<InstallProgress>) {
        if !self.remainder.is_empty() {
            let line = self.remainder.clone();
            self.remainder.clear();
            self.process_line(&line, progress_tx);
        }

        if self.saw_download {
            if !self.sent_extracting {
                let _ = progress_tx.try_send(InstallProgress::Extracting);
                self.sent_extracting = true;
            }
            if !self.sent_configuring {
                let _ = progress_tx.try_send(InstallProgress::Configuring);
                self.sent_configuring = true;
            }
        }
    }

    fn process_line(&mut self, line: &str, progress_tx: &mpsc::Sender<InstallProgress>) {
        let cleaned = sanitize_terminal_text(line);
        if cleaned.is_empty() {
            return;
        }

        let Some((downloaded_bytes, total_bytes)) = parse_download_progress(&cleaned) else {
            return;
        };

        self.saw_download = true;
        if self.last_downloaded_bytes != Some(downloaded_bytes)
            || self.last_total_bytes != Some(total_bytes)
        {
            let _ = progress_tx.try_send(InstallProgress::Downloading {
                downloaded_bytes,
                total_bytes,
            });
            self.last_downloaded_bytes = Some(downloaded_bytes);
            self.last_total_bytes = Some(total_bytes);
        }

        if downloaded_bytes >= total_bytes && !self.sent_extracting {
            let _ = progress_tx.try_send(InstallProgress::Extracting);
            self.sent_extracting = true;
        }
    }
}

async fn read_stream<R>(
    mut stream: R,
    parse_progress: bool,
    progress_tx: mpsc::Sender<InstallProgress>,
) -> Result<(Vec<u8>, Option<InstallProgressParser>), std::io::Error>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    let mut parser = parse_progress.then(InstallProgressParser::default);
    let mut chunk = [0_u8; 4096];

    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }

        bytes.extend_from_slice(&chunk[..read]);
        if let Some(parser) = parser.as_mut() {
            parser.feed_bytes(&chunk[..read], &progress_tx);
        }
    }

    Ok((bytes, parser))
}

fn combined_error_output(stderr: &str, stdout: &str) -> String {
    let stderr = stderr.trim();
    let stdout = stdout.trim();

    if stderr.is_empty() {
        return stdout.to_string();
    }
    if stdout.is_empty() {
        return stderr.to_string();
    }

    format!("{stderr}\n{stdout}")
}

fn parse_download_progress(line: &str) -> Option<(u64, u64)> {
    let slash_index = line.find('/')?;
    let before = line[..slash_index].trim_end();
    let after = line[slash_index + 1..].trim_start();

    let mut before_tokens = before.split_whitespace();
    let downloaded_unit = before_tokens.next_back()?;
    let downloaded_value = before_tokens.next_back()?;

    let mut after_tokens = after.split_whitespace();
    let total_value = after_tokens.next()?;
    let total_unit = after_tokens.next()?;

    let downloaded = parse_size(downloaded_value, downloaded_unit)?;
    let total = parse_size(total_value, total_unit)?;
    if total == 0 {
        return None;
    }

    Some((downloaded.min(total), total))
}

fn parse_size(value: &str, unit: &str) -> Option<u64> {
    let (scaled_value, scale) = parse_decimal_scaled(value)?;
    let multiplier = match unit.trim_end_matches([',', ')', ']']) {
        "B" => 1_u128,
        "KB" | "KiB" => 1024_u128,
        "MB" | "MiB" => 1024_u128.pow(2),
        "GB" | "GiB" => 1024_u128.pow(3),
        "TB" | "TiB" => 1024_u128.pow(4),
        _ => return None,
    };

    let scaled_bytes = scaled_value.checked_mul(multiplier)?;
    let rounded = scaled_bytes.checked_add(scale / 2)?.checked_div(scale)?;
    u64::try_from(rounded).ok()
}

fn parse_decimal_scaled(value: &str) -> Option<(u128, u128)> {
    let (whole, frac) = match value.split_once('.') {
        Some((whole, frac)) => (whole, frac),
        None => (value, ""),
    };

    if whole.is_empty() || whole.starts_with('-') || frac.starts_with('-') {
        return None;
    }
    if !whole.chars().all(|ch| ch.is_ascii_digit()) || !frac.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    let whole_part: u128 = whole.parse().ok()?;
    let frac_part: u128 = if frac.is_empty() {
        0
    } else {
        frac.parse().ok()?
    };
    let frac_len_u32 = u32::try_from(frac.len()).ok()?;
    let scale = 10_u128.checked_pow(frac_len_u32)?;

    let scaled = whole_part.checked_mul(scale)?.checked_add(frac_part)?;
    Some((scaled, scale))
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
            supports_uninstall: true,
        }
    }

    fn backend_info(&self) -> &BackendInfo {
        &self.info
    }

    async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError> {
        debug!("fnm: listing installed versions");
        let output = self.execute(&["list"]).await?;
        Ok(parse_installed_versions(&output))
    }

    async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        debug!("fnm: listing remote versions");
        let output = self.execute(&["list-remote"]).await?;
        Ok(parse_remote_versions(&output))
    }

    async fn list_remote_lts(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        debug!("fnm: listing remote LTS versions");
        let output = self.execute(&["list-remote", "--lts"]).await?;
        Ok(parse_remote_versions(&output))
    }

    async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        debug!("fnm: getting current version");
        let output = self.execute(&["current"]).await?;
        parse_current_version(&output)
    }

    async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        debug!("fnm: getting default version");
        let versions = self.list_installed().await?;
        Ok(find_default_version(versions))
    }

    async fn install(&self, version: &str) -> Result<(), BackendError> {
        info!("fnm: installing version {version}");
        self.execute(&["install", version]).await?;
        Ok(())
    }

    async fn install_with_progress(
        &self,
        version: &str,
        progress_tx: mpsc::Sender<InstallProgress>,
    ) -> Result<(), BackendError> {
        self.execute_install_with_progress(version, progress_tx)
            .await
    }

    async fn uninstall(&self, version: &str) -> Result<(), BackendError> {
        info!("fnm: uninstalling version {version}");
        self.execute(&["uninstall", version]).await?;
        Ok(())
    }

    async fn set_default(&self, version: &str) -> Result<(), BackendError> {
        info!("fnm: setting default version to {version}");
        self.execute(&["default", version]).await?;
        Ok(())
    }

    async fn use_version(&self, version: &str) -> Result<(), BackendError> {
        info!("fnm: using version {version}");
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
            "bash" | "zsh" => Some(format!("eval \"$(fnm env{flags_str})\"")),
            "fish" => Some(format!("fnm env{flags_str} | source")),
            "powershell" | "pwsh" => Some(format!(
                "fnm env{flags_str} | Out-String | Invoke-Expression"
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use tokio::sync::mpsc;

    use versi_backend::{
        InstallProgress, ShellInitOptions, VersionManager, sanitize_terminal_text,
    };

    use super::{FnmBackend, parse_download_progress};

    fn backend() -> FnmBackend {
        FnmBackend::new(PathBuf::from("fnm"), Some("1.38.0".to_string()), None)
    }

    #[test]
    fn capabilities_enable_fnm_supported_features() {
        let capabilities = backend().capabilities();

        assert!(capabilities.supports_lts_filter);
        assert!(capabilities.supports_use_version);
        assert!(capabilities.supports_shell_integration);
        assert!(capabilities.supports_auto_switch);
        assert!(capabilities.supports_corepack);
        assert!(capabilities.supports_resolve_engines);
        assert!(capabilities.supports_uninstall);
    }

    #[test]
    fn shell_init_command_builds_bash_flags() {
        let options = ShellInitOptions {
            use_on_cd: true,
            resolve_engines: true,
            corepack_enabled: false,
        };

        let command = backend()
            .shell_init_command("bash", &options)
            .expect("bash init command should be supported");

        assert_eq!(command, "eval \"$(fnm env --use-on-cd --resolve-engines)\"");
    }

    #[test]
    fn shell_init_command_builds_fish_command() {
        let options = ShellInitOptions {
            use_on_cd: false,
            resolve_engines: false,
            corepack_enabled: true,
        };

        let command = backend()
            .shell_init_command("fish", &options)
            .expect("fish init command should be supported");

        assert_eq!(command, "fnm env --corepack-enabled | source");
    }

    #[test]
    fn shell_init_command_returns_none_for_unknown_shell() {
        let options = ShellInitOptions::default();

        assert!(backend().shell_init_command("nu", &options).is_none());
    }

    #[test]
    fn sanitize_terminal_text_removes_ansi_and_backspaces() {
        let raw = "\x1b[2K^D\x08\x0800:00:00 \u{2588} 10.49 MiB/19.66 MiB (4.23 MiB/s, 2s)\r";
        let cleaned = sanitize_terminal_text(raw);
        assert_eq!(
            cleaned,
            "00:00:00 \u{2588} 10.49 MiB/19.66 MiB (4.23 MiB/s, 2s)"
        );
    }

    #[test]
    fn parse_download_progress_extracts_downloaded_and_total_bytes() {
        let line = "00:00:03 \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{258c} 10.71 MiB/19.66 MiB (4.20 MiB/s, 2s)";
        let (downloaded, total) =
            parse_download_progress(line).expect("progress line should parse");

        assert_eq!(downloaded, 11_230_249);
        assert_eq!(total, 20_615_004);
    }

    #[test]
    fn parser_emits_downloading_extracting_and_configuring() {
        let (tx, mut rx) = mpsc::channel(8);
        let mut parser = super::InstallProgressParser::default();

        parser.feed_bytes(b"00:00:00  1.00 MiB/2.00 MiB (4.00 MiB/s, 1s)\r", &tx);
        parser.feed_bytes(b"00:00:01  2.00 MiB/2.00 MiB (4.00 MiB/s, 0s)\r", &tx);
        parser.finish(&tx);

        let first = rx.try_recv().expect("first progress message");
        let second = rx.try_recv().expect("second progress message");
        let third = rx.try_recv().expect("third progress message");
        let fourth = rx.try_recv().expect("fourth progress message");

        assert!(matches!(
            first,
            InstallProgress::Downloading {
                downloaded_bytes,
                total_bytes
            } if downloaded_bytes == 1_048_576 && total_bytes == 2_097_152
        ));
        assert!(matches!(
            second,
            InstallProgress::Downloading {
                downloaded_bytes,
                total_bytes
            } if downloaded_bytes == 2_097_152 && total_bytes == 2_097_152
        ));
        assert!(matches!(third, InstallProgress::Extracting));
        assert!(matches!(fourth, InstallProgress::Configuring));
    }
}
