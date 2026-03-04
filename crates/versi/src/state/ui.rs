use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub version: String,
    pub is_installed: bool,
    pub is_default: bool,
    pub position: iced::Point,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: usize,
    pub message: String,
    pub created_at: Instant,
}

impl Toast {
    pub fn error(id: usize, message: String) -> Self {
        Self {
            id,
            message,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self, timeout_secs: u64) -> bool {
        self.created_at.elapsed().as_secs() > timeout_secs
    }
}

#[derive(Debug, Clone)]
pub struct SettingsModalState {
    pub shell_statuses: Vec<ShellSetupStatus>,
    pub checking_shells: bool,
    pub log_file_size: Option<u64>,
    pub log_file_path: String,
}

impl SettingsModalState {
    pub fn new() -> Self {
        let log_file_path = versi_platform::AppPaths::new()
            .map(|paths| paths.log_file().to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            shell_statuses: Vec::new(),
            checking_shells: false,
            log_file_size: None,
            log_file_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShellSetupStatus {
    pub shell_type: versi_shell::ShellType,
    pub shell_name: String,
    pub status: ShellVerificationStatus,
    pub configuring: bool,
}

#[derive(Debug, Clone)]
pub enum ShellVerificationStatus {
    Configured,
    NotConfigured,
    NoConfigFile,
    FunctionalButNotInConfig,
    Error,
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{SettingsModalState, Toast};

    #[test]
    fn toast_error_sets_id_and_message() {
        let toast = Toast::error(7, "operation failed".to_string());

        assert_eq!(toast.id, 7);
        assert_eq!(toast.message, "operation failed");
    }

    #[test]
    fn toast_expiration_respects_timeout_boundary() {
        let fresh = Toast {
            id: 1,
            message: "fresh".to_string(),
            created_at: Instant::now(),
        };
        assert!(!fresh.is_expired(0));

        let stale = Toast {
            id: 2,
            message: "stale".to_string(),
            created_at: Instant::now()
                .checked_sub(Duration::from_secs(2))
                .expect("constructing stale toast timestamp should not underflow"),
        };
        assert!(stale.is_expired(1));
    }

    #[test]
    fn settings_modal_state_new_starts_empty() {
        let state = SettingsModalState::new();

        assert!(state.shell_statuses.is_empty());
        assert!(!state.checking_shells);
        assert!(state.log_file_size.is_none());
    }
}
