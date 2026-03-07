use std::collections::{HashSet, VecDeque};

use versi_backend::NodeVersion;

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    Install { version: NodeVersion },
    Uninstall { version: NodeVersion },
    SetDefault { version: NodeVersion },
}

impl Operation {
    pub fn version(&self) -> NodeVersion {
        match self {
            Self::Install { version }
            | Self::Uninstall { version }
            | Self::SetDefault { version } => *version,
        }
    }
}

#[derive(Clone)]
pub struct OperationQueue {
    pub active_installs: Vec<Operation>,
    pub exclusive_op: Option<Operation>,
    pub pending: VecDeque<Operation>,
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

    pub fn has_pending_for_version(&self, version: NodeVersion) -> bool {
        self.pending.iter().any(|op| op.version() == version)
    }

    pub fn is_current_version(&self, version: NodeVersion) -> bool {
        self.active_installs
            .iter()
            .any(|op| op.version() == version)
            || self
                .exclusive_op
                .as_ref()
                .is_some_and(|op| op.version() == version)
    }

    pub fn active_operation_for(&self, version: NodeVersion) -> Option<&Operation> {
        self.active_installs
            .iter()
            .find(|op| op.version() == version)
            .or_else(|| {
                self.exclusive_op
                    .as_ref()
                    .filter(|op| op.version() == version)
            })
    }

    pub fn has_active_install(&self, version: NodeVersion) -> bool {
        self.active_installs
            .iter()
            .any(|op| matches!(op, Operation::Install { .. }) && op.version() == version)
    }

    pub fn enqueue(&mut self, op: Operation) {
        self.pending.push_back(op);
    }

    pub fn remove_pending_matching(
        &mut self,
        mut predicate: impl FnMut(&Operation) -> bool,
    ) -> Vec<Operation> {
        let mut removed = Vec::new();
        let mut kept = VecDeque::with_capacity(self.pending.len());

        while let Some(op) = self.pending.pop_front() {
            if predicate(&op) {
                removed.push(op);
            } else {
                kept.push_back(op);
            }
        }

        self.pending = kept;
        removed
    }

    pub fn start_install(&mut self, version: NodeVersion) {
        self.active_installs.push(Operation::Install { version });
    }

    pub fn start_exclusive(&mut self, op: Operation) {
        self.exclusive_op = Some(op);
    }

    pub fn complete_exclusive(&mut self) {
        self.exclusive_op = None;
    }

    pub fn remove_completed_install(&mut self, version: NodeVersion) {
        self.active_installs.retain(|op| op.version() != version);
    }

    pub fn drain_next(&mut self) -> (Vec<NodeVersion>, Option<Operation>) {
        self.drain_next_with_limit(None)
    }

    pub fn drain_next_with_limit(
        &mut self,
        max_install_starts: Option<usize>,
    ) -> (Vec<NodeVersion>, Option<Operation>) {
        let mut install_versions: Vec<NodeVersion> = Vec::new();
        let mut queued_installs: HashSet<NodeVersion> = HashSet::new();
        let mut exclusive_op: Option<Operation> = None;
        let install_limit = max_install_starts.unwrap_or(usize::MAX);

        if self.exclusive_op.is_some() {
            return (install_versions, exclusive_op);
        }

        let mut skip_count = 0;
        while let Some(next) = self.pending.get(skip_count) {
            if let Operation::Install { version } = next {
                if install_versions.len() >= install_limit {
                    break;
                }

                if self.has_active_install(*version) {
                    skip_count += 1;
                    continue;
                }

                if queued_installs.insert(*version) {
                    install_versions.push(*version);
                }
                self.pending.remove(skip_count);
            } else {
                if self.active_installs.is_empty() && install_versions.is_empty() && skip_count == 0
                {
                    exclusive_op = self.pending.pop_front();
                }
                break;
            }
        }

        (install_versions, exclusive_op)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn nv(major: u32, minor: u32, patch: u32) -> NodeVersion {
        NodeVersion::new(major, minor, patch)
    }

    fn version_tag(tag: u8) -> NodeVersion {
        nv(u32::from(tag), 0, 0)
    }

    fn make_op(kind: u8, tag: u8) -> Operation {
        let version = version_tag(tag);
        match kind % 3 {
            0 => Operation::Install { version },
            1 => Operation::Uninstall { version },
            _ => Operation::SetDefault { version },
        }
    }

    fn generate_pending(seed: u64, len: usize) -> Vec<(u8, u8)> {
        let mut state = seed;
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let kind = ((state >> 8) % 3) as u8;
            let tag = ((state >> 16) % 20) as u8;
            out.push((kind, tag));
        }
        out
    }

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
        q.start_install(nv(20, 0, 0));
        assert!(!q.is_busy_for_install());
    }

    #[test]
    fn is_busy_for_install_with_exclusive_op() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: nv(18, 0, 0),
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
        q.start_install(nv(20, 0, 0));
        assert!(q.is_busy_for_exclusive());
    }

    #[test]
    fn is_busy_for_exclusive_with_exclusive_op() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::SetDefault {
            version: nv(20, 0, 0),
        });
        assert!(q.is_busy_for_exclusive());
    }

    #[test]
    fn is_busy_for_exclusive_with_both() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        q.start_exclusive(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        assert!(q.is_busy_for_exclusive());
    }

    #[test]
    fn has_pending_for_version_empty() {
        let q = OperationQueue::new();
        assert!(!q.has_pending_for_version(nv(20, 0, 0)));
    }

    #[test]
    fn has_pending_for_version_match() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        assert!(q.has_pending_for_version(nv(20, 0, 0)));
        assert!(!q.has_pending_for_version(nv(18, 0, 0)));
    }

    #[test]
    fn has_pending_for_version_with_exclusive_request() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        assert!(q.has_pending_for_version(nv(18, 0, 0)));
    }

    #[test]
    fn is_current_version_empty() {
        let q = OperationQueue::new();
        assert!(!q.is_current_version(nv(20, 0, 0)));
    }

    #[test]
    fn is_current_version_in_active_installs() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        assert!(q.is_current_version(nv(20, 0, 0)));
        assert!(!q.is_current_version(nv(18, 0, 0)));
    }

    #[test]
    fn is_current_version_in_exclusive_uninstall() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        assert!(q.is_current_version(nv(18, 0, 0)));
        assert!(!q.is_current_version(nv(20, 0, 0)));
    }

    #[test]
    fn is_current_version_in_exclusive_set_default() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::SetDefault {
            version: nv(20, 0, 0),
        });
        assert!(q.is_current_version(nv(20, 0, 0)));
    }

    #[test]
    fn active_operation_for_empty() {
        let q = OperationQueue::new();
        assert!(q.active_operation_for(nv(20, 0, 0)).is_none());
    }

    #[test]
    fn active_operation_for_active_install() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        let op = q.active_operation_for(nv(20, 0, 0));
        assert!(matches!(
            op,
            Some(Operation::Install { version }) if *version == nv(20, 0, 0)
        ));
    }

    #[test]
    fn active_operation_for_exclusive() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        let op = q.active_operation_for(nv(18, 0, 0));
        assert!(matches!(
            op,
            Some(Operation::Uninstall { version }) if *version == nv(18, 0, 0)
        ));
    }

    #[test]
    fn active_operation_for_prefers_active_install_over_exclusive() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        q.start_exclusive(Operation::SetDefault {
            version: nv(20, 0, 0),
        });
        let op = q.active_operation_for(nv(20, 0, 0));
        assert!(matches!(op, Some(Operation::Install { .. })));
    }

    #[test]
    fn has_active_install_empty() {
        let q = OperationQueue::new();
        assert!(!q.has_active_install(nv(20, 0, 0)));
    }

    #[test]
    fn has_active_install_present() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        assert!(q.has_active_install(nv(20, 0, 0)));
        assert!(!q.has_active_install(nv(18, 0, 0)));
    }

    #[test]
    fn enqueue_adds_to_pending() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.enqueue(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        assert_eq!(q.pending.len(), 2);
    }

    #[test]
    fn remove_pending_matching_removes_only_selected_operations() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.enqueue(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        q.enqueue(Operation::Install {
            version: nv(22, 0, 0),
        });

        let removed = q.remove_pending_matching(
            |op| matches!(op, Operation::Install { version } if *version == nv(20, 0, 0)),
        );

        assert_eq!(removed.len(), 1);
        assert!(matches!(
            removed.first(),
            Some(Operation::Install { version }) if *version == nv(20, 0, 0)
        ));
        assert_eq!(q.pending.len(), 2);
        assert!(matches!(
            q.pending.front(),
            Some(Operation::Uninstall { version }) if *version == nv(18, 0, 0)
        ));
    }

    #[test]
    fn start_install_adds_to_active() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        assert_eq!(q.active_installs.len(), 1);
        assert!(
            matches!(&q.active_installs[0], Operation::Install { version } if *version == nv(20, 0, 0))
        );
    }

    #[test]
    fn start_exclusive_sets_op() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        assert!(q.exclusive_op.is_some());
    }

    #[test]
    fn complete_exclusive_clears_op() {
        let mut q = OperationQueue::new();
        q.start_exclusive(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        q.complete_exclusive();
        assert!(q.exclusive_op.is_none());
    }

    #[test]
    fn remove_completed_install_removes_matching() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        q.start_install(nv(18, 0, 0));
        q.remove_completed_install(nv(20, 0, 0));
        assert_eq!(q.active_installs.len(), 1);
        assert!(q.has_active_install(nv(18, 0, 0)));
        assert!(!q.has_active_install(nv(20, 0, 0)));
    }

    #[test]
    fn remove_completed_install_no_op_when_missing() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        q.remove_completed_install(nv(18, 0, 0));
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
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.start_exclusive(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(exclusive.is_none());
        assert_eq!(q.pending.len(), 1);
    }

    #[test]
    fn drain_next_drains_all_pending_installs() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.enqueue(Operation::Install {
            version: nv(18, 0, 0),
        });
        let (installs, exclusive) = q.drain_next();
        assert_eq!(installs, vec![nv(20, 0, 0), nv(18, 0, 0)]);
        assert!(exclusive.is_none());
        assert!(q.pending.is_empty());
    }

    #[test]
    fn drain_next_deduplicates_same_version_installs() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        let (installs, _) = q.drain_next();
        assert_eq!(installs, vec![nv(20, 0, 0)]);
    }

    #[test]
    fn drain_next_with_limit_starts_only_limited_installs() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.enqueue(Operation::Install {
            version: nv(18, 0, 0),
        });
        q.enqueue(Operation::Install {
            version: nv(22, 0, 0),
        });

        let (installs, exclusive) = q.drain_next_with_limit(Some(1));

        assert_eq!(installs, vec![nv(20, 0, 0)]);
        assert!(exclusive.is_none());
        assert_eq!(q.pending.len(), 2);
        assert!(matches!(
            q.pending.front(),
            Some(Operation::Install { version }) if *version == nv(18, 0, 0)
        ));
    }

    #[test]
    fn drain_next_skips_already_active_install() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.enqueue(Operation::Install {
            version: nv(18, 0, 0),
        });
        let (installs, _) = q.drain_next();
        assert_eq!(installs, vec![nv(18, 0, 0)]);
    }

    #[test]
    fn drain_next_extracts_exclusive_when_no_installs_active() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(
            matches!(exclusive, Some(Operation::Uninstall { version }) if version == nv(18, 0, 0))
        );
        assert!(q.pending.is_empty());
    }

    #[test]
    fn drain_next_installs_before_exclusive_stops_at_exclusive() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.enqueue(Operation::Uninstall {
            version: nv(18, 0, 0),
        });
        let (installs, exclusive) = q.drain_next();
        assert_eq!(installs, vec![nv(20, 0, 0)]);
        assert!(exclusive.is_none());
        assert_eq!(q.pending.len(), 1);
    }

    #[test]
    fn drain_next_exclusive_blocked_by_active_installs() {
        let mut q = OperationQueue::new();
        q.start_install(nv(20, 0, 0));
        q.enqueue(Operation::SetDefault {
            version: nv(20, 0, 0),
        });
        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(exclusive.is_none());
        assert_eq!(q.pending.len(), 1);
    }

    #[test]
    fn drain_next_set_default_as_exclusive() {
        let mut q = OperationQueue::new();
        q.enqueue(Operation::SetDefault {
            version: nv(20, 0, 0),
        });
        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(
            matches!(exclusive, Some(Operation::SetDefault { version }) if version == nv(20, 0, 0))
        );
    }

    #[test]
    fn full_lifecycle_install() {
        let mut q = OperationQueue::new();

        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.enqueue(Operation::SetDefault {
            version: nv(20, 0, 0),
        });

        let (installs, exclusive) = q.drain_next();
        assert_eq!(installs, vec![nv(20, 0, 0)]);
        assert!(exclusive.is_none());

        for v in &installs {
            q.start_install(*v);
        }
        assert!(q.has_active_install(nv(20, 0, 0)));
        assert!(q.is_busy_for_exclusive());

        q.remove_completed_install(nv(20, 0, 0));
        assert!(!q.has_active_install(nv(20, 0, 0)));

        let (installs, exclusive) = q.drain_next();
        assert!(installs.is_empty());
        assert!(
            matches!(&exclusive, Some(Operation::SetDefault { version }) if *version == nv(20, 0, 0))
        );

        if let Some(op) = exclusive {
            q.start_exclusive(op);
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
        q.enqueue(Operation::Install {
            version: nv(20, 0, 0),
        });
        q.enqueue(Operation::Install {
            version: nv(18, 0, 0),
        });
        q.enqueue(Operation::Install {
            version: nv(22, 0, 0),
        });

        let (installs, _) = q.drain_next();
        assert_eq!(installs.len(), 3);
        for v in &installs {
            q.start_install(*v);
        }

        q.remove_completed_install(nv(18, 0, 0));
        assert_eq!(q.active_installs.len(), 2);
        assert!(q.has_active_install(nv(20, 0, 0)));
        assert!(q.has_active_install(nv(22, 0, 0)));
        assert!(!q.has_active_install(nv(18, 0, 0)));

        q.remove_completed_install(nv(20, 0, 0));
        q.remove_completed_install(nv(22, 0, 0));
        assert!(q.active_installs.is_empty());
        assert!(!q.is_busy_for_exclusive());
    }

    #[test]
    fn drain_next_preserves_invariants_across_generated_queue_states() {
        for active_mask in 0u16..32 {
            for exclusive_variant in 0u8..4 {
                for seed in 0u64..64 {
                    for len in 0usize..16 {
                        let mut queue = OperationQueue::new();

                        for bit in 0..5u8 {
                            if (active_mask & (1 << bit)) != 0 {
                                queue.start_install(version_tag(bit));
                            }
                        }

                        if exclusive_variant != 0 {
                            queue
                                .start_exclusive(make_op(exclusive_variant - 1, (seed % 20) as u8));
                        }

                        for (kind, tag) in generate_pending(seed, len) {
                            queue.enqueue(make_op(kind, tag));
                        }

                        let had_exclusive = queue.exclusive_op.is_some();
                        let had_active_installs = !queue.active_installs.is_empty();
                        let pending_len_before = queue.pending.len();
                        let active_installs_before = queue.active_installs.clone();

                        let (installs, exclusive_request) = queue.drain_next();

                        let unique_installs: HashSet<&NodeVersion> = installs.iter().collect();
                        assert_eq!(installs.len(), unique_installs.len());

                        for version in &installs {
                            assert!(!active_installs_before.iter().any(
                                |op| matches!(op, Operation::Install { version: active } if active == version)
                            ));
                        }

                        if had_exclusive {
                            assert!(installs.is_empty());
                            assert!(exclusive_request.is_none());
                            assert_eq!(queue.pending.len(), pending_len_before);
                        }

                        if let Some(request) = exclusive_request {
                            assert!(installs.is_empty());
                            assert!(!matches!(request, Operation::Install { .. }));
                            assert!(!had_active_installs);
                        }
                    }
                }
            }
        }
    }
}
