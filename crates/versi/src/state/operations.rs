use std::collections::VecDeque;

use versi_backend::InstallProgress;

#[derive(Debug, Clone)]
pub enum Operation {
    Install {
        version: String,
        progress: InstallProgress,
    },
    Uninstall {
        version: String,
    },
    SetDefault {
        version: String,
    },
}

#[derive(Debug, Clone)]
pub enum OperationRequest {
    Install { version: String },
    Uninstall { version: String },
    SetDefault { version: String },
}

impl OperationRequest {
    pub fn version(&self) -> &str {
        match self {
            Self::Install { version } => version,
            Self::Uninstall { version } => version,
            Self::SetDefault { version } => version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueuedOperation {
    pub request: OperationRequest,
}

#[derive(Clone)]
pub struct OperationQueue {
    pub active_installs: Vec<Operation>,
    pub exclusive_op: Option<Operation>,
    pub pending: VecDeque<QueuedOperation>,
}

impl std::fmt::Debug for OperationQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationQueue")
            .field("active_installs", &self.active_installs.len())
            .field("exclusive_op", &self.exclusive_op)
            .field("pending", &self.pending.len())
            .finish()
    }
}

impl Default for OperationQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationQueue {
    pub fn new() -> Self {
        Self {
            active_installs: Vec::new(),
            exclusive_op: None,
            pending: VecDeque::new(),
        }
    }

    pub fn is_busy_for_install(&self) -> bool {
        self.exclusive_op.is_some()
    }

    pub fn is_busy_for_exclusive(&self) -> bool {
        !self.active_installs.is_empty() || self.exclusive_op.is_some()
    }

    pub fn has_pending_for_version(&self, version: &str) -> bool {
        self.pending
            .iter()
            .any(|op| op.request.version() == version)
    }

    pub fn is_current_version(&self, version: &str) -> bool {
        let in_installs = self.active_installs.iter().any(|op| match op {
            Operation::Install { version: v, .. } => v == version,
            _ => false,
        });
        if in_installs {
            return true;
        }
        self.exclusive_op
            .as_ref()
            .map(|op| match op {
                Operation::Install { version: v, .. } => v == version,
                Operation::Uninstall { version: v } => v == version,
                Operation::SetDefault { version: v } => v == version,
            })
            .unwrap_or(false)
    }

    pub fn active_operation_for(&self, version: &str) -> Option<&Operation> {
        if let Some(op) = self
            .active_installs
            .iter()
            .find(|op| matches!(op, Operation::Install { version: v, .. } if v == version))
        {
            return Some(op);
        }
        self.exclusive_op.as_ref().filter(|op| match op {
            Operation::Install { version: v, .. } => v == version,
            Operation::Uninstall { version: v } => v == version,
            Operation::SetDefault { version: v } => v == version,
        })
    }

    pub fn remove_completed_install(&mut self, version: &str) {
        self.active_installs.retain(|op| match op {
            Operation::Install { version: v, .. } => v != version,
            _ => true,
        });
    }

    pub fn update_install_progress(&mut self, version: &str, progress: InstallProgress) {
        if let Some(Operation::Install {
            progress: op_progress,
            ..
        }) = self
            .active_installs
            .iter_mut()
            .find(|op| matches!(op, Operation::Install { version: v, .. } if v == version))
        {
            *op_progress = progress;
        }
    }
}

#[derive(Debug, Clone)]
pub enum Modal {
    ConfirmBulkUpdateMajors {
        versions: Vec<(String, String)>,
    },
    ConfirmBulkUninstallEOL {
        versions: Vec<String>,
    },
    ConfirmBulkUninstallMajor {
        major: u32,
        versions: Vec<String>,
    },
    ConfirmBulkUninstallMajorExceptLatest {
        major: u32,
        versions: Vec<String>,
        keeping: String,
    },
}
