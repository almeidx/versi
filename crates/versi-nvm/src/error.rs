use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum NvmError {
    #[error("nvm not found")]
    NotFound,

    #[error("Command failed: {stderr}")]
    CommandFailed { stderr: String },

    #[error("Failed to parse version: {0}")]
    ParseError(String),

    #[error("Installation failed: {0}")]
    InstallFailed(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Timeout waiting for command")]
    Timeout,
}

impl From<std::io::Error> for NvmError {
    fn from(err: std::io::Error) -> Self {
        NvmError::IoError(err.to_string())
    }
}
