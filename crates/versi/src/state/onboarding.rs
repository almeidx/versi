use std::path::PathBuf;

use versi_shell::ShellType;

use crate::backend_kind::BackendKind;
use crate::error::AppError;

#[derive(Debug)]
pub struct OnboardingState {
    pub step: OnboardingStep,
    pub backend_installing: bool,
    pub confirming_unsafe_install: bool,
    pub install_error: Option<AppError>,
    pub detected_shells: Vec<ShellConfigStatus>,
    pub available_backends: Vec<BackendOption>,
    pub selected_backend: Option<BackendKind>,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Welcome,
            backend_installing: false,
            confirming_unsafe_install: false,
            install_error: None,
            detected_shells: Vec::new(),
            available_backends: Vec::new(),
            selected_backend: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{OnboardingState, OnboardingStep};

    #[test]
    fn onboarding_state_new_has_expected_defaults() {
        let state = OnboardingState::new();

        assert_eq!(state.step, OnboardingStep::Welcome);
        assert!(!state.backend_installing);
        assert!(!state.confirming_unsafe_install);
        assert!(state.install_error.is_none());
        assert!(state.detected_shells.is_empty());
        assert!(state.available_backends.is_empty());
        assert!(state.selected_backend.is_none());
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    Welcome,
    SelectBackend,
    InstallBackend,
    ConfigureShell,
}

#[derive(Debug, Clone)]
pub struct ShellConfigStatus {
    pub shell_type: ShellType,
    pub shell_name: String,
    pub configured: bool,
    pub config_path: Option<PathBuf>,
    pub configuring: bool,
    pub error: Option<AppError>,
}

#[derive(Debug, Clone)]
pub struct BackendOption {
    pub kind: BackendKind,
    pub display_name: &'static str,
    pub detected: bool,
}
