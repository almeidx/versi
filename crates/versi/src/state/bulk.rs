use versi_backend::NodeVersion;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BulkRunKind {
    UpdateMajors,
    UninstallEol,
    UninstallMajor,
    UninstallMajorExceptLatest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BulkRunAction {
    Install,
    Uninstall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkItemStatus {
    Pending,
    Running,
    Completed,
    Failed(AppError),
    Canceled,
}

impl BulkItemStatus {
    fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }

    #[cfg(test)]
    fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }

    #[cfg(test)]
    fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    #[cfg(test)]
    fn is_canceled(&self) -> bool {
        matches!(self, Self::Canceled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkRunItem {
    pub version: NodeVersion,
    pub action: BulkRunAction,
    pub status: BulkItemStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkRunState {
    pub kind: BulkRunKind,
    pub items: Vec<BulkRunItem>,
}

impl BulkRunState {
    #[must_use]
    pub fn new(kind: BulkRunKind, items: Vec<BulkRunItem>) -> Self {
        Self { kind, items }
    }

    fn items_matching(
        &self,
        predicate: fn(&BulkItemStatus) -> bool,
    ) -> impl Iterator<Item = &BulkRunItem> {
        self.items
            .iter()
            .filter(move |item| predicate(&item.status))
    }

    #[must_use]
    pub fn total_count(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.items_matching(BulkItemStatus::is_pending).count()
    }

    #[must_use]
    pub fn running_count(&self) -> usize {
        self.items_matching(BulkItemStatus::is_running).count()
    }

    #[cfg(test)]
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.items_matching(BulkItemStatus::is_completed).count()
    }

    #[cfg(test)]
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.items_matching(BulkItemStatus::is_failed).count()
    }

    #[cfg(test)]
    #[must_use]
    pub fn canceled_count(&self) -> usize {
        self.items_matching(BulkItemStatus::is_canceled).count()
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.pending_count() > 0 || self.running_count() > 0
    }

    #[cfg(test)]
    #[must_use]
    pub fn completed_versions(&self) -> Vec<NodeVersion> {
        self.items_matching(BulkItemStatus::is_completed)
            .map(|item| item.version)
            .collect()
    }

    #[cfg(test)]
    #[must_use]
    pub fn failed_versions(&self) -> Vec<NodeVersion> {
        self.items_matching(BulkItemStatus::is_failed)
            .map(|item| item.version)
            .collect()
    }

    #[cfg(test)]
    #[must_use]
    pub fn canceled_versions(&self) -> Vec<NodeVersion> {
        self.items_matching(BulkItemStatus::is_canceled)
            .map(|item| item.version)
            .collect()
    }

    fn find_item_mut(
        &mut self,
        version: &NodeVersion,
        action: BulkRunAction,
    ) -> Option<&mut BulkRunItem> {
        self.items
            .iter_mut()
            .find(|item| item.version == *version && item.action == action)
    }

    pub fn mark_running(&mut self, version: &NodeVersion, action: BulkRunAction) {
        if let Some(item) = self.find_item_mut(version, action)
            && matches!(item.status, BulkItemStatus::Pending)
        {
            item.status = BulkItemStatus::Running;
        }
    }

    pub fn mark_finished(
        &mut self,
        version: &NodeVersion,
        action: BulkRunAction,
        success: bool,
        error: Option<AppError>,
    ) {
        if let Some(item) = self.find_item_mut(version, action) {
            if matches!(item.status, BulkItemStatus::Canceled) {
                return;
            }

            item.status = if success {
                BulkItemStatus::Completed
            } else {
                BulkItemStatus::Failed(error.unwrap_or_else(|| {
                    AppError::operation_failed("Bulk operation", "unknown error")
                }))
            };
        }
    }

    pub fn cancel_pending(&mut self) -> Vec<(NodeVersion, BulkRunAction)> {
        let mut canceled = Vec::new();
        for item in &mut self.items {
            if matches!(item.status, BulkItemStatus::Pending) {
                item.status = BulkItemStatus::Canceled;
                canceled.push((item.version, item.action));
            }
        }
        canceled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nv(major: u32, minor: u32, patch: u32) -> NodeVersion {
        NodeVersion::new(major, minor, patch)
    }

    #[test]
    fn bulk_run_state_tracks_status_transitions() {
        let mut run = BulkRunState::new(
            BulkRunKind::UpdateMajors,
            vec![
                BulkRunItem {
                    version: nv(22, 1, 0),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Pending,
                },
                BulkRunItem {
                    version: nv(20, 11, 1),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Pending,
                },
                BulkRunItem {
                    version: nv(18, 20, 0),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Pending,
                },
            ],
        );

        run.mark_running(&nv(22, 1, 0), BulkRunAction::Install);
        run.mark_finished(&nv(22, 1, 0), BulkRunAction::Install, true, None);

        run.mark_running(&nv(20, 11, 1), BulkRunAction::Install);
        run.mark_finished(
            &nv(20, 11, 1),
            BulkRunAction::Install,
            false,
            Some(AppError::operation_failed("Install", "boom")),
        );

        let canceled = run.cancel_pending();

        assert_eq!(run.total_count(), 3);
        assert_eq!(run.pending_count(), 0);
        assert_eq!(run.running_count(), 0);
        assert_eq!(run.completed_count(), 1);
        assert_eq!(run.failed_count(), 1);
        assert_eq!(run.canceled_count(), 1);
        assert!(!run.is_active());
        assert_eq!(canceled, vec![(nv(18, 20, 0), BulkRunAction::Install)]);
        assert_eq!(run.completed_versions(), vec![nv(22, 1, 0)]);
        assert_eq!(run.failed_versions(), vec![nv(20, 11, 1)]);
        assert_eq!(run.canceled_versions(), vec![nv(18, 20, 0)]);
    }

    #[test]
    fn items_matching_filters_by_status() {
        let run = BulkRunState::new(
            BulkRunKind::UninstallEol,
            vec![
                BulkRunItem {
                    version: nv(14, 0, 0),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Completed,
                },
                BulkRunItem {
                    version: nv(16, 0, 0),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Pending,
                },
                BulkRunItem {
                    version: nv(12, 0, 0),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Failed(AppError::operation_failed(
                        "Uninstall",
                        "permission denied",
                    )),
                },
                BulkRunItem {
                    version: nv(10, 0, 0),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Running,
                },
                BulkRunItem {
                    version: nv(8, 0, 0),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Canceled,
                },
            ],
        );

        let pending: Vec<NodeVersion> = run
            .items_matching(BulkItemStatus::is_pending)
            .map(|item| item.version)
            .collect();
        assert_eq!(pending, vec![nv(16, 0, 0)]);

        let running: Vec<NodeVersion> = run
            .items_matching(BulkItemStatus::is_running)
            .map(|item| item.version)
            .collect();
        assert_eq!(running, vec![nv(10, 0, 0)]);

        let completed: Vec<NodeVersion> = run
            .items_matching(BulkItemStatus::is_completed)
            .map(|item| item.version)
            .collect();
        assert_eq!(completed, vec![nv(14, 0, 0)]);

        let failed: Vec<NodeVersion> = run
            .items_matching(BulkItemStatus::is_failed)
            .map(|item| item.version)
            .collect();
        assert_eq!(failed, vec![nv(12, 0, 0)]);

        let canceled: Vec<NodeVersion> = run
            .items_matching(BulkItemStatus::is_canceled)
            .map(|item| item.version)
            .collect();
        assert_eq!(canceled, vec![nv(8, 0, 0)]);
    }

    #[test]
    fn items_matching_returns_empty_for_no_matches() {
        let run = BulkRunState::new(
            BulkRunKind::UninstallEol,
            vec![BulkRunItem {
                version: nv(14, 0, 0),
                action: BulkRunAction::Uninstall,
                status: BulkItemStatus::Pending,
            }],
        );

        assert_eq!(run.items_matching(BulkItemStatus::is_completed).count(), 0);
        assert_eq!(run.items_matching(BulkItemStatus::is_failed).count(), 0);
        assert_eq!(run.items_matching(BulkItemStatus::is_canceled).count(), 0);
        assert_eq!(run.items_matching(BulkItemStatus::is_running).count(), 0);
    }
}
