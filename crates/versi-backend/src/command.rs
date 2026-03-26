use std::path::PathBuf;
use std::process::Output;

use log::{debug, error, trace};
use tokio::process::Command;
use versi_core::HideWindow;

use crate::error::BackendError;

#[derive(Debug, Clone)]
pub enum CommandEnvironment {
    Native { binary_path: PathBuf },
    Wsl { distro: String, binary_path: String },
}

impl CommandEnvironment {
    #[must_use]
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native { .. })
    }
}

#[must_use]
pub fn build_backend_command(env: &CommandEnvironment, args: &[&str]) -> Command {
    match env {
        CommandEnvironment::Native { binary_path } => {
            let mut cmd = Command::new(binary_path);
            cmd.args(args);
            cmd.hide_window();
            cmd
        }
        CommandEnvironment::Wsl {
            distro,
            binary_path,
        } => {
            let mut cmd = Command::new("wsl.exe");
            cmd.args(["-d", distro, "--", binary_path]);
            cmd.args(args);
            cmd.hide_window();
            cmd
        }
    }
}

/// Build, run, and log a backend command, returning its stdout on success.
///
/// # Errors
///
/// Returns [`BackendError::CommandFailed`] when the process exits
/// with a non-zero status, or a transport error if spawning fails.
pub async fn execute_backend_command(
    backend_name: &str,
    env: &CommandEnvironment,
    args: &[&str],
) -> Result<String, BackendError> {
    execute_backend_command_with(backend_name, env, args, |_| {}).await
}

/// Like [`execute_backend_command`], but calls `configure` on the [`Command`]
/// before spawning, allowing callers to inject environment variables.
///
/// # Errors
///
/// Returns [`BackendError::CommandFailed`] when the process exits
/// with a non-zero status, or a transport error if spawning fails.
pub async fn execute_backend_command_with(
    backend_name: &str,
    env: &CommandEnvironment,
    args: &[&str],
    configure: impl FnOnce(&mut Command),
) -> Result<String, BackendError> {
    debug!(
        "Executing {backend_name} command: {} {}",
        match env {
            CommandEnvironment::Native { binary_path } => binary_path.display().to_string(),
            CommandEnvironment::Wsl {
                distro,
                binary_path,
            } => format!("wsl.exe -d {distro} -- {binary_path}"),
        },
        args.join(" ")
    );

    let mut cmd = build_backend_command(env, args);
    cmd.kill_on_drop(true);
    configure(&mut cmd);
    let output = cmd.output().await?;

    debug!("{backend_name} command exit status: {:?}", output.status);
    trace!(
        "{backend_name} stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    if !output.stderr.is_empty() {
        trace!(
            "{backend_name} stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    command_output_to_result(&output).inspect_err(|err| {
        error!("{backend_name} command failed: args={args:?}, error='{err}'");
    })
}

#[must_use]
pub fn combine_error_output(stderr: &str, stdout: &str) -> String {
    let stderr = stderr.trim();
    let stdout = stdout.trim();

    match (stderr.is_empty(), stdout.is_empty()) {
        (true, _) => stdout.to_string(),
        (_, true) => stderr.to_string(),
        _ => format!("{stderr}\n{stdout}"),
    }
}

/// Convert a finished [`Output`] into a `Result<String, BackendError>`.
///
/// On success stdout is returned as-is (callers that need further
/// sanitization can post-process).  On failure stderr and stdout are merged
/// so that backends which emit diagnostics on either stream are fully
/// represented in the error.
///
/// # Errors
///
/// Returns [`BackendError::CommandFailed`] when the process exited with a
/// non-zero status.
pub fn command_output_to_result(output: &Output) -> Result<String, BackendError> {
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    Err(BackendError::CommandFailed {
        stderr: combine_error_output(&stderr, &stdout),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Output;

    use super::{CommandEnvironment, build_backend_command, command_output_to_result};
    use crate::BackendError;

    fn exit_status(code: i32) -> std::process::ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(code << 8)
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(code.unsigned_abs())
        }
    }

    fn output(code: i32, stdout: &[u8], stderr: &[u8]) -> Output {
        Output {
            status: exit_status(code),
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
    }

    #[test]
    fn success_returns_stdout() {
        let result = command_output_to_result(&output(0, b"v20.11.0\n", b""));
        assert_eq!(result.unwrap(), "v20.11.0\n");
    }

    #[test]
    fn failure_with_only_stderr() {
        let result = command_output_to_result(&output(1, b"", b"not found\n"));
        assert_eq!(
            result.unwrap_err(),
            BackendError::CommandFailed {
                stderr: "not found".to_string()
            }
        );
    }

    #[test]
    fn failure_with_only_stdout() {
        let result = command_output_to_result(&output(1, b"error detail\n", b""));
        assert_eq!(
            result.unwrap_err(),
            BackendError::CommandFailed {
                stderr: "error detail".to_string()
            }
        );
    }

    #[test]
    fn failure_combines_stderr_and_stdout() {
        let result = command_output_to_result(&output(1, b"stdout info\n", b"stderr info\n"));
        assert_eq!(
            result.unwrap_err(),
            BackendError::CommandFailed {
                stderr: "stderr info\nstdout info".to_string()
            }
        );
    }

    #[test]
    fn build_native_command_uses_binary_path() {
        let env = CommandEnvironment::Native {
            binary_path: PathBuf::from("/usr/local/bin/fnm"),
        };
        let cmd = build_backend_command(&env, &["list"]);
        let program = format!("{:?}", cmd.as_std().get_program());
        assert!(program.contains("fnm"));
    }

    #[test]
    fn build_wsl_command_uses_wsl_exe() {
        let env = CommandEnvironment::Wsl {
            distro: "Ubuntu".to_string(),
            binary_path: "/home/user/.local/bin/fnm".to_string(),
        };
        let cmd = build_backend_command(&env, &["list"]);
        let program = format!("{:?}", cmd.as_std().get_program());
        assert!(program.contains("wsl"));
    }

    #[test]
    fn is_native_returns_true_for_native() {
        let env = CommandEnvironment::Native {
            binary_path: PathBuf::from("fnm"),
        };
        assert!(env.is_native());
    }

    #[test]
    fn is_native_returns_false_for_wsl() {
        let env = CommandEnvironment::Wsl {
            distro: "Ubuntu".to_string(),
            binary_path: "fnm".to_string(),
        };
        assert!(!env.is_native());
    }
}
