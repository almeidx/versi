use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum Operation {
    Install { version: String },
    Uninstall { version: String },
    SetDefault { version: String },
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

    pub fn has_active_install(&self, version: &str) -> bool {
        self.active_installs
            .iter()
            .any(|op| matches!(op, Operation::Install { version: v, .. } if v == version))
    }

    pub fn enqueue(&mut self, request: OperationRequest) {
        self.pending.push_back(QueuedOperation { request });
    }

    pub fn start_install(&mut self, version: String) {
        self.active_installs.push(Operation::Install { version });
    }

    pub fn start_exclusive(&mut self, op: Operation) {
        self.exclusive_op = Some(op);
    }

    pub fn complete_exclusive(&mut self) {
        self.exclusive_op = None;
    }

    pub fn remove_completed_install(&mut self, version: &str) {
        self.active_installs.retain(|op| match op {
            Operation::Install { version: v, .. } => v != version,
            _ => true,
        });
    }

    pub fn drain_next(&mut self) -> (Vec<String>, Option<OperationRequest>) {
        let mut install_versions: Vec<String> = Vec::new();
        let mut exclusive_request: Option<OperationRequest> = None;

        if self.exclusive_op.is_some() {
            return (install_versions, exclusive_request);
        }

        while let Some(next) = self.pending.front() {
            match &next.request {
                OperationRequest::Install { version } => {
                    if !self.has_active_install(version) && !install_versions.contains(version) {
                        install_versions.push(version.clone());
                    }
                    self.pending.pop_front();
                }
                _ => {
                    if self.active_installs.is_empty()
                        && install_versions.is_empty()
                        && let Some(queued) = self.pending.pop_front()
                    {
                        exclusive_request = Some(queued.request);
                    }
                    break;
                }
            }
        }

        (install_versions, exclusive_request)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_queue_is_empty() {
        let q = OperationQueue::new();
        assert!(q.active_installs.is_empty());
        assert!(q.exclusive_op.is_none());
        assert!(q.pending.is_empty());
    }

    #[test]
    fn default_matches_new() {
        let q = OperationQueue::default();
        assert!(q.active_installs.is_empty());
        assert!(q.exclusive_op.is_none());
        assert!(q.pending.is_empty());
    }

    #[test]
    fn is_busy_for_install_when_empty() {
        let q = OperationQueue::new();
        assert!(!q.is_busy_for_install());
    }

    #[test]
    fn is_busy_for_install_with_active_installs_only() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        assert!(!q.is_busy_for_install());
    }

    #[test]
    fn is_busy_for_install_with_exclusive_op() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: "18.0.0".into(),
        });
        assert!(q.is_busy_for_install());
    }

    #[test]
    fn is_busy_for_exclusive_when_empty() {
        let q = OperationQueue::new();
        assert!(!q.is_busy_for_exclusive());
    }

    #[test]
    fn is_busy_for_exclusive_with_active_installs() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        assert!(q.is_busy_for_exclusive());
    }

    #[test]
    fn is_busy_for_exclusive_with_exclusive_op() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::SetDefault {
            version: "20.0.0".into(),
        });
        assert!(q.is_busy_for_exclusive());
    }

    #[test]
    fn is_busy_for_exclusive_with_both() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        q.start_exclusive(Operation::Uninstall {
            version: "18.0.0".into(),
        });
        assert!(q.is_busy_for_exclusive());
    }

    #[test]
    fn has_pending_for_version_empty() {
        let q = OperationQueue::new();
        assert!(!q.has_pending_for_version("20.0.0"));
    }

    #[test]
    fn has_pending_for_version_match() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        assert!(q.has_pending_for_version("20.0.0"));
        assert!(!q.has_pending_for_version("18.0.0"));
    }

    #[test]
    fn has_pending_for_version_with_exclusive_request() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::Uninstall {
            version: "18.0.0".into(),
        });
        assert!(q.has_pending_for_version("18.0.0"));
    }

    #[test]
    fn is_current_version_empty() {
        let q = OperationQueue::new();
        assert!(!q.is_current_version("20.0.0"));
    }

    #[test]
    fn is_current_version_in_active_installs() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        assert!(q.is_current_version("20.0.0"));
        assert!(!q.is_current_version("18.0.0"));
    }

    #[test]
    fn is_current_version_in_exclusive_uninstall() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: "18.0.0".into(),
        });
        assert!(q.is_current_version("18.0.0"));
        assert!(!q.is_current_version("20.0.0"));
    }

    #[test]
    fn is_current_version_in_exclusive_set_default() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::SetDefault {
            version: "20.0.0".into(),
        });
        assert!(q.is_current_version("20.0.0"));
    }

    #[test]
    fn active_operation_for_empty() {
        let q = OperationQueue::new();
        assert!(q.active_operation_for("20.0.0").is_none());
    }

    #[test]
    fn active_operation_for_active_install() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        let op = q.active_operation_for("20.0.0");
        assert!(matches!(
            op,
            Some(Operation::Install { version, .. }) if version == "20.0.0"
        ));
    }

    #[test]
    fn active_operation_for_exclusive() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: "18.0.0".into(),
        });
        let op = q.active_operation_for("18.0.0");
        assert!(matches!(
            op,
            Some(Operation::Uninstall { version }) if version == "18.0.0"
        ));
    }

    #[test]
    fn active_operation_for_prefers_active_install_over_exclusive() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        q.start_exclusive(Operation::SetDefault {
            version: "20.0.0".into(),
        });
        let op = q.active_operation_for("20.0.0");
        assert!(matches!(op, Some(Operation::Install { .. })));
    }

    #[test]
    fn has_active_install_empty() {
        let q = OperationQueue::new();
        assert!(!q.has_active_install("20.0.0"));
    }

    #[test]
    fn has_active_install_present() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        assert!(q.has_active_install("20.0.0"));
        assert!(!q.has_active_install("18.0.0"));
    }

    #[test]
    fn enqueue_adds_to_pending() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        q.enqueue(OperationRequest::Uninstall {
            version: "18.0.0".into(),
        });
        assert_eq!(q.pending.len(), 2);
    }

    #[test]
    fn start_install_adds_to_active() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        assert_eq!(q.active_installs.len(), 1);
        assert!(
            matches!(&q.active_installs[0], Operation::Install { version } if version == "20.0.0")
        );
    }

    #[test]
    fn start_exclusive_sets_op() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: "18.0.0".into(),
        });
        assert!(q.exclusive_op.is_some());
    }

    #[test]
    fn complete_exclusive_clears_op() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: "18.0.0".into(),
        });
        q.complete_exclusive();
        assert!(q.exclusive_op.is_none());
    }

    #[test]
    fn remove_completed_install_removes_matching() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        q.start_install("18.0.0".into());
        q.remove_completed_install("20.0.0");
        assert_eq!(q.active_installs.len(), 1);
        assert!(q.has_active_install("18.0.0"));
        assert!(!q.has_active_install("20.0.0"));
    }

    #[test]
    fn remove_completed_install_no_op_when_missing() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        q.remove_completed_install("18.0.0");
        assert_eq!(q.active_installs.len(), 1);
    }

    #[test]
    fn drain_next_empty_queue() {
        let mut q = OperationQueue::new();
        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(exclusive.is_none());
    }

    #[test]
    fn drain_next_returns_early_when_exclusive_active() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        q.start_exclusive(Operation::Uninstall {
            version: "18.0.0".into(),
        });
        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(exclusive.is_none());
        assert_eq!(q.pending.len(), 1);
    }

    #[test]
    fn drain_next_drains_all_pending_installs() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        q.enqueue(OperationRequest::Install {
            version: "18.0.0".into(),
        });
        let (installs, exclusive) = q.drain_next();
        assert_eq!(installs, vec!["20.0.0", "18.0.0"]);
        assert!(exclusive.is_none());
        assert!(q.pending.is_empty());
    }

    #[test]
    fn drain_next_deduplicates_same_version_installs() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        let (installs, _) = q.drain_next();
        assert_eq!(installs, vec!["20.0.0"]);
    }

    #[test]
    fn drain_next_skips_already_active_install() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        q.enqueue(OperationRequest::Install {
            version: "18.0.0".into(),
        });
        let (installs, _) = q.drain_next();
        assert_eq!(installs, vec!["18.0.0"]);
    }

    #[test]
    fn drain_next_extracts_exclusive_when_no_installs_active() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::Uninstall {
            version: "18.0.0".into(),
        });
        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(
            matches!(exclusive, Some(OperationRequest::Uninstall { version }) if version == "18.0.0")
        );
        assert!(q.pending.is_empty());
    }

    #[test]
    fn drain_next_installs_before_exclusive_stops_at_exclusive() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        q.enqueue(OperationRequest::Uninstall {
            version: "18.0.0".into(),
        });
        let (installs, exclusive) = q.drain_next();
        assert_eq!(installs, vec!["20.0.0"]);
        assert!(exclusive.is_none());
        assert_eq!(q.pending.len(), 1);
    }

    #[test]
    fn drain_next_exclusive_blocked_by_active_installs() {
        let mut q = OperationQueue::new();
        q.start_install("20.0.0".into());
        q.enqueue(OperationRequest::SetDefault {
            version: "20.0.0".into(),
        });
        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(exclusive.is_none());
        assert_eq!(q.pending.len(), 1);
    }

    #[test]
    fn drain_next_set_default_as_exclusive() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::SetDefault {
            version: "20.0.0".into(),
        });
        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(
            matches!(exclusive, Some(OperationRequest::SetDefault { version }) if version == "20.0.0")
        );
    }

    #[test]
    fn full_lifecycle_install() {
        let mut q = OperationQueue::new();

        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        q.enqueue(OperationRequest::SetDefault {
            version: "20.0.0".into(),
        });

        let (installs, exclusive) = q.drain_next();
        assert_eq!(installs, vec!["20.0.0"]);
        assert!(exclusive.is_none());

        for v in &installs {
            q.start_install(v.clone());
        }
        assert!(q.has_active_install("20.0.0"));
        assert!(q.is_busy_for_exclusive());

        q.remove_completed_install("20.0.0");
        assert!(!q.has_active_install("20.0.0"));

        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(
            matches!(&exclusive, Some(OperationRequest::SetDefault { version }) if version == "20.0.0")
        );

        if let Some(req) = exclusive {
            q.start_exclusive(Operation::SetDefault {
                version: match &req {
                    OperationRequest::SetDefault { version } => version.clone(),
                    _ => unreachable!(),
                },
            });
        }
        assert!(q.is_busy_for_install());
        assert!(q.is_busy_for_exclusive());

        q.complete_exclusive();
        assert!(!q.is_busy_for_install());
        assert!(!q.is_busy_for_exclusive());
        assert!(q.pending.is_empty());
    }

    #[test]
    fn full_lifecycle_concurrent_installs() {
        let mut q = OperationQueue::new();
        q.enqueue(OperationRequest::Install {
            version: "20.0.0".into(),
        });
        q.enqueue(OperationRequest::Install {
            version: "18.0.0".into(),
        });
        q.enqueue(OperationRequest::Install {
            version: "22.0.0".into(),
        });

        let (installs, _) = q.drain_next();
        assert_eq!(installs.len(), 3);
        for v in &installs {
            q.start_install(v.clone());
        }

        q.remove_completed_install("18.0.0");
        assert_eq!(q.active_installs.len(), 2);
        assert!(q.has_active_install("20.0.0"));
        assert!(q.has_active_install("22.0.0"));
        assert!(!q.has_active_install("18.0.0"));

        q.remove_completed_install("20.0.0");
        q.remove_completed_install("22.0.0");
        assert!(q.active_installs.is_empty());
        assert!(!q.is_busy_for_exclusive());
    }
}
