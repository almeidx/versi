use std::process::Output;

use crate::error::BackendError;

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
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let combined = match (stderr.is_empty(), stdout.is_empty()) {
        (true, _) => stdout,
        (_, true) => stderr,
        _ => format!("{stderr}\n{stdout}"),
    };

    Err(BackendError::CommandFailed { stderr: combined })
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    use super::command_output_to_result;
    use crate::BackendError;

    fn output(code: i32, stdout: &[u8], stderr: &[u8]) -> Output {
        Output {
            status: ExitStatus::from_raw(code << 8),
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
}
