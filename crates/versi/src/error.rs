#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreErrorKind {
    AutoUpdate,
    Metadata,
    Schedule,
    SecurityAdvisory,
    UpdateCheck,
}

impl std::fmt::Display for CoreErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AutoUpdate => write!(f, "auto-update"),
            Self::Metadata => write!(f, "metadata"),
            Self::Schedule => write!(f, "schedule"),
            Self::SecurityAdvisory => write!(f, "security advisory"),
            Self::UpdateCheck => write!(f, "update check"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppErrorDetail {
    Message(String),
    Io {
        kind: std::io::ErrorKind,
        message: String,
    },
    Backend(versi_backend::BackendError),
    ShellVerification(versi_shell::VerificationError),
    Core {
        kind: CoreErrorKind,
        message: String,
    },
}

impl std::fmt::Display for AppErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => write!(f, "{message}"),
            Self::Io { kind, message } => write!(f, "{kind}: {message}"),
            Self::Backend(error) => write!(f, "{error}"),
            Self::ShellVerification(error) => write!(f, "{error}"),
            Self::Core { kind, message } => write!(f, "{kind}: {message}"),
        }
    }
}

impl From<String> for AppErrorDetail {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for AppErrorDetail {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<std::io::Error> for AppErrorDetail {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            kind: error.kind(),
            message: error.to_string(),
        }
    }
}

impl From<versi_backend::BackendError> for AppErrorDetail {
    fn from(value: versi_backend::BackendError) -> Self {
        Self::Backend(value)
    }
}

impl From<versi_shell::VerificationError> for AppErrorDetail {
    fn from(value: versi_shell::VerificationError) -> Self {
        Self::ShellVerification(value)
    }
}

impl From<versi_shell::ConfigError> for AppErrorDetail {
    fn from(value: versi_shell::ConfigError) -> Self {
        Self::ShellVerification(versi_shell::VerificationError::ConfigLoad(value.into()))
    }
}

impl From<versi_shell::ShellConfigLoadError> for AppErrorDetail {
    fn from(value: versi_shell::ShellConfigLoadError) -> Self {
        Self::ShellVerification(versi_shell::VerificationError::ConfigLoad(value))
    }
}

impl From<versi_shell::WslShellConfigError> for AppErrorDetail {
    fn from(value: versi_shell::WslShellConfigError) -> Self {
        Self::ShellVerification(versi_shell::VerificationError::Wsl(value))
    }
}

impl From<serde_json::Error> for AppErrorDetail {
    fn from(error: serde_json::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<versi_core::auto_update::AutoUpdateError> for AppErrorDetail {
    fn from(value: versi_core::auto_update::AutoUpdateError) -> Self {
        Self::Core {
            kind: CoreErrorKind::AutoUpdate,
            message: value.to_string(),
        }
    }
}

impl From<versi_core::MetadataError> for AppErrorDetail {
    fn from(value: versi_core::MetadataError) -> Self {
        Self::Core {
            kind: CoreErrorKind::Metadata,
            message: value.to_string(),
        }
    }
}

impl From<versi_core::ScheduleError> for AppErrorDetail {
    fn from(value: versi_core::ScheduleError) -> Self {
        Self::Core {
            kind: CoreErrorKind::Schedule,
            message: value.to_string(),
        }
    }
}

impl From<versi_core::SecurityAdvisoryError> for AppErrorDetail {
    fn from(value: versi_core::SecurityAdvisoryError) -> Self {
        Self::Core {
            kind: CoreErrorKind::SecurityAdvisory,
            message: value.to_string(),
        }
    }
}

impl From<versi_core::UpdateError> for AppErrorDetail {
    fn from(value: versi_core::UpdateError) -> Self {
        Self::Core {
            kind: CoreErrorKind::UpdateCheck,
            message: value.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Timeout {
        operation: &'static str,
        seconds: u64,
    },
    ShellConfigPathNotFound {
        shell: &'static str,
    },
    ShellNotSupported {
        shell: &'static str,
    },
    ShellConfigFailed {
        shell: &'static str,
        action: &'static str,
        details: AppErrorDetail,
    },
    BackendInstallFailed {
        backend: &'static str,
        details: AppErrorDetail,
    },
    SettingsDialogCancelled,
    SettingsExportFailed {
        action: &'static str,
        details: AppErrorDetail,
    },
    SettingsImportFailed {
        action: &'static str,
        details: AppErrorDetail,
    },
    OperationFailed {
        operation: &'static str,
        details: AppErrorDetail,
    },
    OperationCancelled {
        operation: &'static str,
    },
    EnvironmentUnavailable {
        reason: AppErrorDetail,
    },
    EnvironmentLoadFailed {
        details: AppErrorDetail,
    },
    VersionFetchFailed {
        resource: &'static str,
        details: AppErrorDetail,
    },
    AutoUpdateFailed {
        phase: &'static str,
        details: AppErrorDetail,
    },
    UpdateCheckFailed {
        target: &'static str,
        details: AppErrorDetail,
    },
}

impl AppError {
    pub fn timeout(operation: &'static str, seconds: u64) -> Self {
        Self::Timeout { operation, seconds }
    }

    pub fn shell_config_path_not_found(shell: &'static str) -> Self {
        Self::ShellConfigPathNotFound { shell }
    }

    pub fn shell_not_supported(shell: &'static str) -> Self {
        Self::ShellNotSupported { shell }
    }

    pub fn shell_config_failed(
        shell: &'static str,
        action: &'static str,
        details: impl Into<AppErrorDetail>,
    ) -> Self {
        Self::ShellConfigFailed {
            shell,
            action,
            details: details.into(),
        }
    }

    pub fn backend_install_failed(
        backend: &'static str,
        details: impl Into<AppErrorDetail>,
    ) -> Self {
        Self::BackendInstallFailed {
            backend,
            details: details.into(),
        }
    }

    pub fn settings_dialog_cancelled() -> Self {
        Self::SettingsDialogCancelled
    }

    pub fn settings_export_failed(
        action: &'static str,
        details: impl Into<AppErrorDetail>,
    ) -> Self {
        Self::SettingsExportFailed {
            action,
            details: details.into(),
        }
    }

    pub fn settings_import_failed(
        action: &'static str,
        details: impl Into<AppErrorDetail>,
    ) -> Self {
        Self::SettingsImportFailed {
            action,
            details: details.into(),
        }
    }

    pub fn operation_failed(operation: &'static str, details: impl Into<AppErrorDetail>) -> Self {
        Self::OperationFailed {
            operation,
            details: details.into(),
        }
    }

    pub fn operation_cancelled(operation: &'static str) -> Self {
        Self::OperationCancelled { operation }
    }

    pub fn environment_unavailable(reason: impl Into<AppErrorDetail>) -> Self {
        Self::EnvironmentUnavailable {
            reason: reason.into(),
        }
    }

    pub fn environment_load_failed(details: impl Into<AppErrorDetail>) -> Self {
        Self::EnvironmentLoadFailed {
            details: details.into(),
        }
    }

    pub fn version_fetch_failed(
        resource: &'static str,
        details: impl Into<AppErrorDetail>,
    ) -> Self {
        Self::VersionFetchFailed {
            resource,
            details: details.into(),
        }
    }

    pub fn auto_update_failed(phase: &'static str, details: impl Into<AppErrorDetail>) -> Self {
        Self::AutoUpdateFailed {
            phase,
            details: details.into(),
        }
    }

    pub fn update_check_failed(target: &'static str, details: impl Into<AppErrorDetail>) -> Self {
        Self::UpdateCheckFailed {
            target,
            details: details.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { operation, seconds } => {
                write!(f, "{operation} timed out after {seconds}s")
            }
            Self::ShellConfigPathNotFound { shell } => {
                write!(f, "No shell config file path found for {shell}")
            }
            Self::ShellNotSupported { shell } => write!(f, "{shell} shell is not supported"),
            Self::ShellConfigFailed {
                shell,
                action,
                details,
            } => write!(f, "{shell} shell {action} failed: {details}"),
            Self::BackendInstallFailed { backend, details } => {
                write!(f, "Failed to install backend {backend}: {details}")
            }
            Self::SettingsDialogCancelled => write!(f, "Cancelled"),
            Self::SettingsExportFailed { action, details } => {
                write!(f, "Settings export {action} failed: {details}")
            }
            Self::SettingsImportFailed { action, details } => {
                write!(f, "Settings import {action} failed: {details}")
            }
            Self::OperationFailed { operation, details } => {
                write!(f, "{operation} failed: {details}")
            }
            Self::OperationCancelled { operation } => write!(f, "{operation} cancelled"),
            Self::EnvironmentUnavailable { reason } => write!(f, "{reason}"),
            Self::EnvironmentLoadFailed { details } => {
                write!(f, "Failed to load versions: {details}")
            }
            Self::VersionFetchFailed { resource, details } => {
                write!(f, "{resource} fetch failed: {details}")
            }
            Self::AutoUpdateFailed { phase, details } => {
                write!(f, "App update {phase} failed: {details}")
            }
            Self::UpdateCheckFailed { target, details } => {
                write!(f, "{target} update check failed: {details}")
            }
        }
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::{AppError, AppErrorDetail, CoreErrorKind};

    #[test]
    fn timeout_constructor_and_display_match() {
        let error = AppError::timeout("install", 30);
        assert_eq!(
            error,
            AppError::Timeout {
                operation: "install",
                seconds: 30
            }
        );
        assert_eq!(error.to_string(), "install timed out after 30s");
    }

    #[test]
    fn shell_error_constructors_include_context() {
        let missing = AppError::shell_config_path_not_found("Bash");
        let unsupported = AppError::shell_not_supported("Fish");
        let failed = AppError::shell_config_failed("Zsh", "load config", "permission denied");

        assert_eq!(missing, AppError::ShellConfigPathNotFound { shell: "Bash" });
        assert_eq!(unsupported, AppError::ShellNotSupported { shell: "Fish" });
        assert_eq!(
            failed,
            AppError::ShellConfigFailed {
                shell: "Zsh",
                action: "load config",
                details: AppErrorDetail::from("permission denied")
            }
        );
        assert_eq!(
            missing.to_string(),
            "No shell config file path found for Bash"
        );
        assert_eq!(unsupported.to_string(), "Fish shell is not supported");
        assert_eq!(
            failed.to_string(),
            "Zsh shell load config failed: permission denied"
        );
    }

    #[test]
    fn backend_install_failed_constructor_includes_backend_name() {
        let detail = versi_backend::BackendError::CommandFailed {
            stderr: "permission denied".to_string(),
        };
        let error = AppError::backend_install_failed("fnm", detail);

        assert_eq!(
            error,
            AppError::BackendInstallFailed {
                backend: "fnm",
                details: AppErrorDetail::Backend(versi_backend::BackendError::CommandFailed {
                    stderr: "permission denied".to_string()
                })
            }
        );
        assert_eq!(
            error.to_string(),
            "Failed to install backend fnm: Command failed: permission denied"
        );
    }

    #[test]
    fn settings_and_cancellation_constructors_are_structured() {
        let cancelled = AppError::settings_dialog_cancelled();
        let export = AppError::settings_export_failed("write file", "permission denied");
        let import = AppError::settings_import_failed("parse json", "invalid type");
        let op_failed = AppError::operation_failed("Install", "backend reported failure");
        let op_cancelled = AppError::operation_cancelled("Remote versions fetch");

        assert_eq!(cancelled, AppError::SettingsDialogCancelled);
        assert_eq!(
            export,
            AppError::SettingsExportFailed {
                action: "write file",
                details: AppErrorDetail::from("permission denied")
            }
        );
        assert_eq!(
            import,
            AppError::SettingsImportFailed {
                action: "parse json",
                details: AppErrorDetail::from("invalid type")
            }
        );
        assert_eq!(
            op_failed,
            AppError::OperationFailed {
                operation: "Install",
                details: AppErrorDetail::from("backend reported failure")
            }
        );
        assert_eq!(
            op_cancelled,
            AppError::OperationCancelled {
                operation: "Remote versions fetch"
            }
        );
        assert_eq!(cancelled.to_string(), "Cancelled");
        assert_eq!(
            export.to_string(),
            "Settings export write file failed: permission denied"
        );
        assert_eq!(
            import.to_string(),
            "Settings import parse json failed: invalid type"
        );
        assert_eq!(
            op_failed.to_string(),
            "Install failed: backend reported failure"
        );
        assert_eq!(op_cancelled.to_string(), "Remote versions fetch cancelled");
    }

    #[test]
    fn fetch_and_environment_error_constructors_include_context() {
        let unavailable = AppError::environment_unavailable("backend unavailable");
        let env_load = AppError::environment_load_failed("backend unavailable");
        let fetch = AppError::version_fetch_failed("Release schedule", "network timeout");
        let update = AppError::auto_update_failed("restart", "spawn failed");
        let update_check = AppError::update_check_failed("App", "rate limited");

        assert_eq!(
            unavailable,
            AppError::EnvironmentUnavailable {
                reason: AppErrorDetail::from("backend unavailable")
            }
        );

        assert_eq!(
            env_load,
            AppError::EnvironmentLoadFailed {
                details: AppErrorDetail::from("backend unavailable")
            }
        );
        assert_eq!(
            fetch,
            AppError::VersionFetchFailed {
                resource: "Release schedule",
                details: AppErrorDetail::from("network timeout")
            }
        );
        assert_eq!(
            update,
            AppError::AutoUpdateFailed {
                phase: "restart",
                details: AppErrorDetail::from("spawn failed")
            }
        );
        assert_eq!(
            update_check,
            AppError::UpdateCheckFailed {
                target: "App",
                details: AppErrorDetail::from("rate limited")
            }
        );
        assert_eq!(unavailable.to_string(), "backend unavailable");
        assert_eq!(
            env_load.to_string(),
            "Failed to load versions: backend unavailable"
        );
        assert_eq!(
            fetch.to_string(),
            "Release schedule fetch failed: network timeout"
        );
        assert_eq!(
            update.to_string(),
            "App update restart failed: spawn failed"
        );
        assert_eq!(
            update_check.to_string(),
            "App update check failed: rate limited"
        );
    }

    #[test]
    fn io_detail_keeps_error_kind_and_message() {
        let detail = AppErrorDetail::from(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        assert!(matches!(
            detail,
            AppErrorDetail::Io {
                kind: std::io::ErrorKind::PermissionDenied,
                ..
            }
        ));
        assert_eq!(detail.to_string(), "permission denied: denied");
    }

    #[test]
    fn core_detail_preserves_kind_and_message() {
        let detail = AppErrorDetail::Core {
            kind: CoreErrorKind::Metadata,
            message: "connection refused".to_string(),
        };
        assert_eq!(
            detail,
            AppErrorDetail::Core {
                kind: CoreErrorKind::Metadata,
                message: "connection refused".to_string(),
            }
        );
        assert_eq!(detail.to_string(), "metadata: connection refused");
    }

    #[test]
    fn core_error_kind_display_covers_all_variants() {
        assert_eq!(CoreErrorKind::AutoUpdate.to_string(), "auto-update");
        assert_eq!(CoreErrorKind::Metadata.to_string(), "metadata");
        assert_eq!(CoreErrorKind::Schedule.to_string(), "schedule");
        assert_eq!(
            CoreErrorKind::SecurityAdvisory.to_string(),
            "security advisory"
        );
        assert_eq!(CoreErrorKind::UpdateCheck.to_string(), "update check");
    }

    #[test]
    fn core_error_kind_is_copy_and_eq() {
        let kind = CoreErrorKind::Schedule;
        let copied = kind;
        assert_eq!(kind, copied);
    }
}
