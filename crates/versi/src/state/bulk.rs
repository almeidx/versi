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

    fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }

    fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    fn is_canceled(&self) -> bool {
        matches!(self, Self::Canceled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkRunItem {
    pub version: String,
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

    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.items_matching(BulkItemStatus::is_completed).count()
    }

    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.items_matching(BulkItemStatus::is_failed).count()
    }

    #[must_use]
    pub fn canceled_count(&self) -> usize {
        self.items_matching(BulkItemStatus::is_canceled).count()
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.pending_count() > 0 || self.running_count() > 0
    }

    #[must_use]
    pub fn pending_versions(&self) -> Vec<String> {
        self.items_matching(BulkItemStatus::is_pending)
            .map(|item| item.version.clone())
            .collect()
    }

    #[must_use]
    pub fn completed_versions(&self) -> Vec<String> {
        self.items_matching(BulkItemStatus::is_completed)
            .map(|item| item.version.clone())
            .collect()
    }

    #[must_use]
    pub fn failed_versions(&self) -> Vec<String> {
        self.items_matching(BulkItemStatus::is_failed)
            .map(|item| item.version.clone())
            .collect()
    }

    #[must_use]
    pub fn canceled_versions(&self) -> Vec<String> {
        self.items_matching(BulkItemStatus::is_canceled)
            .map(|item| item.version.clone())
            .collect()
    }

    fn find_item_mut(&mut self, version: &str, action: BulkRunAction) -> Option<&mut BulkRunItem> {
        self.items
            .iter_mut()
            .find(|item| item.version == version && item.action == action)
    }

    pub fn mark_running(&mut self, version: &str, action: BulkRunAction) {
        if let Some(item) = self.find_item_mut(version, action)
            && matches!(item.status, BulkItemStatus::Pending)
        {
            item.status = BulkItemStatus::Running;
        }
    }

    pub fn mark_finished(
        &mut self,
        version: &str,
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

    pub fn cancel_pending(&mut self) -> Vec<(String, BulkRunAction)> {
        let mut canceled = Vec::new();
        for item in &mut self.items {
            if matches!(item.status, BulkItemStatus::Pending) {
                item.status = BulkItemStatus::Canceled;
                canceled.push((item.version.clone(), item.action));
            }
        }
        canceled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_run_state_tracks_status_transitions() {
        let mut run = BulkRunState::new(
            BulkRunKind::UpdateMajors,
            vec![
                BulkRunItem {
                    version: "v22.1.0".to_string(),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Pending,
                },
                BulkRunItem {
                    version: "v20.11.1".to_string(),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Pending,
                },
                BulkRunItem {
                    version: "v18.20.0".to_string(),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Pending,
                },
            ],
        );

        run.mark_running("v22.1.0", BulkRunAction::Install);
        run.mark_finished("v22.1.0", BulkRunAction::Install, true, None);

        run.mark_running("v20.11.1", BulkRunAction::Install);
        run.mark_finished(
            "v20.11.1",
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
        assert_eq!(
            canceled,
            vec![("v18.20.0".to_string(), BulkRunAction::Install)]
        );
        assert_eq!(run.completed_versions(), vec!["v22.1.0".to_string()]);
        assert_eq!(run.failed_versions(), vec!["v20.11.1".to_string()]);
        assert_eq!(run.canceled_versions(), vec!["v18.20.0".to_string()]);
    }

    #[test]
    fn items_matching_filters_by_status() {
        let run = BulkRunState::new(
            BulkRunKind::UninstallEol,
            vec![
                BulkRunItem {
                    version: "v14.0.0".to_string(),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Completed,
                },
                BulkRunItem {
                    version: "v16.0.0".to_string(),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Pending,
                },
                BulkRunItem {
                    version: "v12.0.0".to_string(),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Failed(AppError::operation_failed(
                        "Uninstall",
                        "permission denied",
                    )),
                },
                BulkRunItem {
                    version: "v10.0.0".to_string(),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Running,
                },
                BulkRunItem {
                    version: "v8.0.0".to_string(),
                    action: BulkRunAction::Uninstall,
                    status: BulkItemStatus::Canceled,
                },
            ],
        );

        let pending: Vec<&str> = run
            .items_matching(BulkItemStatus::is_pending)
            .map(|item| item.version.as_str())
            .collect();
        assert_eq!(pending, vec!["v16.0.0"]);

        let running: Vec<&str> = run
            .items_matching(BulkItemStatus::is_running)
            .map(|item| item.version.as_str())
            .collect();
        assert_eq!(running, vec!["v10.0.0"]);

        let completed: Vec<&str> = run
            .items_matching(BulkItemStatus::is_completed)
            .map(|item| item.version.as_str())
            .collect();
        assert_eq!(completed, vec!["v14.0.0"]);

        let failed: Vec<&str> = run
            .items_matching(BulkItemStatus::is_failed)
            .map(|item| item.version.as_str())
            .collect();
        assert_eq!(failed, vec!["v12.0.0"]);

        let canceled: Vec<&str> = run
            .items_matching(BulkItemStatus::is_canceled)
            .map(|item| item.version.as_str())
            .collect();
        assert_eq!(canceled, vec!["v8.0.0"]);
    }

    #[test]
    fn items_matching_returns_empty_for_no_matches() {
        let run = BulkRunState::new(
            BulkRunKind::UninstallEol,
            vec![BulkRunItem {
                version: "v14.0.0".to_string(),
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
