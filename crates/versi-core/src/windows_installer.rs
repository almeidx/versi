#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallerAttempt {
    pub label: &'static str,
    pub program: &'static str,
    pub args: &'static [&'static str],
}

#[derive(Debug, thiserror::Error)]
pub enum InstallerAttemptError {
    #[error("{label} failed to start: {source}")]
    LaunchFailed {
        label: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("{label} exited with status {code}")]
    NonZeroExit { label: &'static str, code: String },
}

#[cfg(windows)]
pub async fn run_installer_attempt(
    attempt: &InstallerAttempt,
) -> Result<(), InstallerAttemptError> {
    use tokio::process::Command;
    use versi_platform::HideWindow;

    let status = Command::new(attempt.program)
        .args(attempt.args)
        .hide_window()
        .status()
        .await
        .map_err(|source| InstallerAttemptError::LaunchFailed {
            label: attempt.label,
            source,
        })?;

    if status.success() {
        Ok(())
    } else {
        let code = status
            .code()
            .map_or_else(|| "unknown".to_string(), |code| code.to_string());
        Err(InstallerAttemptError::NonZeroExit {
            label: attempt.label,
            code,
        })
    }
}
