use iced::Task;
use log::info;

use crate::message::Message;
use crate::state::{AppState, AppUpdateState, MainViewKind};

use super::super::{Versi, platform};

impl Versi {
    pub(super) fn dispatch_settings(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::ToastDismiss(id) => Ok(self.handle_toast_dismiss(id)),
            Message::NavigateToVersions => Ok(self.navigate_to_versions()),
            Message::NavigateToSettings => Ok(self.navigate_to_settings()),
            Message::NavigateToAbout => Ok(self.navigate_to_about()),
            Message::VersionRowHovered(version) => Ok(self.handle_version_row_hovered(version)),
            Message::ThemeChanged(theme) => Ok(self.handle_theme_changed(theme)),
            Message::AppUpdateBehaviorChanged(behavior) => {
                Ok(self.handle_app_update_behavior_changed(behavior))
            }
            Message::ShellOptionUseOnCdToggled(value) => {
                Ok(self.update_active_shell_options(|options| options.use_on_cd = value))
            }
            Message::ShellOptionResolveEnginesToggled(value) => {
                Ok(self.update_active_shell_options(|options| options.resolve_engines = value))
            }
            Message::ShellOptionCorepackEnabledToggled(value) => {
                Ok(self.update_active_shell_options(|options| options.corepack_enabled = value))
            }
            Message::DebugLoggingToggled(value) => Ok(self.handle_debug_logging_toggled(value)),
            Message::CopyToClipboard(text) => Ok(iced::clipboard::write(text)),
            Message::ClearLogFile => Ok(Self::clear_log_file()),
            Message::LogFileCleared => Ok(self.handle_log_file_cleared()),
            Message::RevealLogFile => Ok(Self::reveal_log_file()),
            Message::RevealSettingsFile => Ok(self.reveal_settings_file()),
            Message::LogFileStatsLoaded(size) => Ok(self.handle_log_file_stats_loaded(size)),
            Message::ShellFlagsUpdated => Ok(Task::none()),
            Message::ExportSettings => Ok(self.handle_export_settings()),
            Message::SettingsExported(result) => Ok(self.handle_settings_exported(result)),
            Message::ImportSettings => Ok(Self::handle_import_settings()),
            Message::SettingsImported(result) => Ok(self.handle_settings_imported(result)),
            Message::ShellSetupChecked(results) => {
                Ok(self.handle_shell_setup_checked_message(results))
            }
            Message::ConfigureShell(shell_type) => Ok(self.handle_configure_shell(shell_type)),
            Message::ShellConfigured(shell_type, result) => {
                Ok(self.handle_shell_configured_message(&shell_type, &result))
            }
            Message::PreferredBackendChanged(name) => {
                Ok(self.handle_preferred_backend_changed(name))
            }
            other => self.dispatch_settings_onboarding_and_system(other),
        }
    }

    fn dispatch_settings_onboarding_and_system(
        &mut self,
        message: Message,
    ) -> super::DispatchResult {
        match message {
            Message::OnboardingNext => Ok(self.handle_onboarding_next()),
            Message::OnboardingBack => {
                self.handle_onboarding_back();
                Ok(Task::none())
            }
            Message::OnboardingSelectBackend(name) => {
                self.handle_onboarding_select_backend(name);
                Ok(Task::none())
            }
            Message::OnboardingInstallBackend => Ok(self.handle_onboarding_install_backend()),
            Message::OnboardingConfirmInstallBackend => {
                Ok(self.handle_onboarding_confirm_install_backend())
            }
            Message::OnboardingCancelInstallBackend => {
                self.handle_onboarding_cancel_install_backend();
                Ok(Task::none())
            }
            Message::OnboardingBackendInstallResult(result) => {
                Ok(self.handle_onboarding_backend_install_result(result))
            }
            Message::OnboardingConfigureShell(shell_type) => {
                Ok(self.handle_onboarding_configure_shell(shell_type))
            }
            Message::OnboardingShellConfigResult(result) => {
                self.handle_onboarding_shell_config_result(&result);
                Ok(Task::none())
            }
            Message::OnboardingComplete => Ok(self.handle_onboarding_complete()),
            Message::TrayBehaviorChanged(behavior) => {
                Ok(self.handle_tray_behavior_changed(behavior))
            }
            Message::StartMinimizedToggled(value) => {
                self.settings.start_minimized = value;
                self.save_settings_with_log();
                Ok(Task::none())
            }
            Message::LaunchAtLoginToggled(value) => Ok(self.handle_launch_at_login_toggled(value)),
            Message::SystemThemeChanged(mode) => {
                self.system_theme_mode = mode;
                Ok(Task::none())
            }
            other => Err(Box::new(other)),
        }
    }

    fn handle_toast_dismiss(&mut self, id: usize) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.remove_toast(id);
        }
        Task::none()
    }

    fn navigate_to_versions(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.view = MainViewKind::Versions;
        }
        Task::none()
    }

    fn navigate_to_settings(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.view = MainViewKind::Settings;
            state.settings_state.checking_shells = true;
        }
        let shell_task = self.handle_check_shell_setup();
        let log_stats_task = load_log_file_stats_task();
        Task::batch([shell_task, log_stats_task])
    }

    fn navigate_to_about(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.view = MainViewKind::About;
        }
        Task::none()
    }

    fn handle_version_row_hovered(&mut self, version: Option<String>) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.modal.is_some() {
                state.hovered_version = None;
            } else {
                state.hovered_version = version;
            }
        }
        Task::none()
    }

    fn handle_theme_changed(&mut self, theme: crate::settings::ThemeSetting) -> Task<Message> {
        self.settings.theme = theme;
        self.save_settings_with_log();
        Task::none()
    }

    fn handle_app_update_behavior_changed(
        &mut self,
        behavior: crate::settings::AppUpdateBehavior,
    ) -> Task<Message> {
        self.settings.app_update_behavior = behavior;
        self.save_settings_with_log();

        if let AppState::Main(state) = &mut self.state
            && behavior == crate::settings::AppUpdateBehavior::DoNotCheck
        {
            state.app_update = None;
            if matches!(
                state.app_update_state,
                AppUpdateState::Idle | AppUpdateState::Failed(_)
            ) {
                state.app_update_state = AppUpdateState::Idle;
            }
            state.app_update_check_in_flight = false;
            return Task::none();
        }

        self.handle_check_for_app_update()
    }

    fn handle_debug_logging_toggled(&mut self, enabled: bool) -> Task<Message> {
        self.settings.debug_logging = enabled;
        self.save_settings_with_log();
        crate::logging::set_logging_enabled(enabled);
        if enabled {
            info!("Debug logging enabled");
        }
        Task::none()
    }

    fn clear_log_file() -> Task<Message> {
        let Some(log_path) = versi_platform::AppPaths::new().ok().map(|p| p.log_file()) else {
            return Task::none();
        };
        Task::perform(
            async move {
                if log_path.exists() {
                    let _ = std::fs::write(&log_path, "");
                }
            },
            |()| Message::LogFileCleared,
        )
    }

    fn handle_log_file_cleared(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.settings_state.log_file_size = Some(0);
        }
        Task::none()
    }

    fn reveal_log_file() -> Task<Message> {
        let Some(log_path) = versi_platform::AppPaths::new().ok().map(|p| p.log_file()) else {
            return Task::none();
        };
        Task::perform(
            async move { platform::reveal_in_file_manager(&log_path) },
            |()| Message::NoOp,
        )
    }

    fn reveal_settings_file(&self) -> Task<Message> {
        self.save_settings_with_log_sync();
        let Some(settings_path) = versi_platform::AppPaths::new()
            .ok()
            .map(|p| p.settings_file())
        else {
            return Task::none();
        };
        Task::perform(
            async move { platform::reveal_in_file_manager(&settings_path) },
            |()| Message::NoOp,
        )
    }

    fn handle_log_file_stats_loaded(&mut self, size: Option<u64>) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.settings_state.log_file_size = size;
        }
        Task::none()
    }

    fn handle_shell_setup_checked_message(
        &mut self,
        results: Vec<(versi_shell::ShellType, versi_shell::VerificationResult)>,
    ) -> Task<Message> {
        self.handle_shell_setup_checked(results);
        Task::none()
    }

    fn handle_shell_configured_message(
        &mut self,
        shell_type: &versi_shell::ShellType,
        result: &Result<(), crate::error::AppError>,
    ) -> Task<Message> {
        self.handle_shell_configured(shell_type, result);
        Task::none()
    }

    fn handle_launch_at_login_toggled(&mut self, value: bool) -> Task<Message> {
        self.settings.launch_at_login = value;
        if let Err(e) = platform::set_launch_at_login(value) {
            log::error!("Failed to set launch at login: {e}");
        }
        self.save_settings_with_log();
        Task::none()
    }

    fn update_active_shell_options<F>(&mut self, update: F) -> Task<Message>
    where
        F: FnOnce(&mut crate::settings::ShellOptions),
    {
        let backend_kind = self.active_backend_kind();
        update(self.settings.shell_options_for_mut(backend_kind));
        self.save_settings_with_log();
        self.update_shell_flags()
    }

    pub(crate) fn save_settings_with_log(&self) {
        super::super::settings_save::enqueue_settings_save(self.settings.clone());
    }

    pub(crate) fn save_settings_with_log_sync(&self) {
        if let Err(e) = self.settings.save() {
            log::error!("Failed to save settings: {e}");
        }
    }
}

fn load_log_file_stats_task() -> Task<Message> {
    Task::perform(
        async {
            let log_path = versi_platform::AppPaths::new().ok()?.log_file();
            std::fs::metadata(&log_path).ok().map(|m| m.len())
        },
        Message::LogFileStatsLoaded,
    )
}

#[cfg(test)]
mod tests {
    use super::super::super::test_app_with_two_environments;
    use super::*;
    use crate::state::{MainViewKind, Modal, Toast};

    #[test]
    fn dispatch_settings_returns_err_for_unhandled_message() {
        let mut app = test_app_with_two_environments();

        let result = app.dispatch_settings(Message::NoOp);

        assert!(matches!(result, Err(other) if matches!(*other, Message::NoOp)));
    }

    #[test]
    fn navigate_to_about_switches_view() {
        let mut app = test_app_with_two_environments();

        let _ = app.dispatch_settings(Message::NavigateToAbout);

        let state = app.main_state();
        assert_eq!(state.view, MainViewKind::About);
    }

    #[test]
    fn version_row_hovered_is_cleared_while_modal_open() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut().modal = Some(Modal::KeyboardShortcuts);

        let _ = app.dispatch_settings(Message::VersionRowHovered(Some("v20.11.0".to_string())));

        let state = app.main_state();
        assert!(state.hovered_version.is_none());
    }

    #[test]
    fn toast_dismiss_removes_matching_toast() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut()
            .toasts
            .push(Toast::error(1, "first".to_string()));
        app.main_state_mut()
            .toasts
            .push(Toast::error(2, "second".to_string()));

        let _ = app.dispatch_settings(Message::ToastDismiss(1));

        let state = app.main_state();
        assert_eq!(state.toasts.len(), 1);
        assert_eq!(state.toasts[0].id, 2);
    }
}
