mod bulk;
mod environment;
mod main;
mod modal;
mod onboarding;
mod operations;
mod ui;

pub use bulk::{BulkItemStatus, BulkRunAction, BulkRunItem, BulkRunKind, BulkRunState};
pub use environment::EnvironmentState;
pub use main::{AppUpdateState, MainState, NetworkStatus, SearchFilter, VersionSecurityFinding};
pub use modal::Modal;
pub use onboarding::{BackendOption, OnboardingState, OnboardingStep, ShellConfigStatus};
pub use operations::{Operation, OperationQueue};
pub use ui::{ContextMenu, SettingsModalState, ShellSetupStatus, ShellVerificationStatus, Toast};

#[derive(Debug)]
pub enum AppState {
    Loading,
    Onboarding(OnboardingState),
    Main(Box<MainState>),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum MainViewKind {
    #[default]
    Versions,
    Settings,
    About,
}

#[cfg(test)]
mod tests {
    use super::MainViewKind;

    #[test]
    fn main_view_kind_default_is_versions() {
        assert_eq!(MainViewKind::default(), MainViewKind::Versions);
    }
}
