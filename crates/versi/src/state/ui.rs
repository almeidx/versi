use std::time::Instant;

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
}

impl SettingsModalState {
    pub fn new() -> Self {
        Self {
            shell_statuses: Vec::new(),
            checking_shells: false,
            log_file_size: None,
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
