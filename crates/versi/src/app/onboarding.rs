use iced::Task;

use crate::backend_kind::BackendKind;
use crate::error::AppError;
use crate::message::Message;
use crate::state::{AppState, OnboardingStep};

use super::Versi;

impl Versi {
    pub(super) fn handle_onboarding_next(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.confirming_unsafe_install = false;
            state.step = match state.step {
                OnboardingStep::Welcome => {
                    if state.available_backends.len() > 1 {
                        OnboardingStep::SelectBackend
                    } else {
                        OnboardingStep::InstallBackend
                    }
                }
                OnboardingStep::SelectBackend => OnboardingStep::InstallBackend,
                OnboardingStep::InstallBackend => OnboardingStep::ConfigureShell,
                OnboardingStep::ConfigureShell => return self.handle_onboarding_complete(),
            };
        }
        Task::none()
    }

    pub(super) fn handle_onboarding_back(&mut self) {
        if let AppState::Onboarding(state) = &mut self.state {
            state.confirming_unsafe_install = false;
            state.step = match state.step {
                OnboardingStep::Welcome | OnboardingStep::SelectBackend => OnboardingStep::Welcome,
                OnboardingStep::InstallBackend => {
                    if state.available_backends.len() > 1 {
                        OnboardingStep::SelectBackend
                    } else {
                        OnboardingStep::Welcome
                    }
                }
                OnboardingStep::ConfigureShell => OnboardingStep::InstallBackend,
            };
        }
    }

    pub(super) fn handle_onboarding_select_backend(&mut self, kind: BackendKind) {
        if let AppState::Onboarding(state) = &mut self.state {
            state.selected_backend = Some(kind);
        }
        self.settings.preferred_backend = Some(kind);
        self.save_settings_with_log();

        if let Some(provider) = self.providers.get(&kind) {
            self.provider = provider.clone();
        }
    }

    pub(super) fn handle_onboarding_install_backend(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.confirming_unsafe_install = true;
        }
        Task::none()
    }

    pub(super) fn handle_onboarding_confirm_install_backend(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.backend_installing = true;
            state.confirming_unsafe_install = false;
            state.install_error = None;

            let provider = self.provider.clone();
            let backend_name = provider.name();
            return Task::perform(
                async move {
                    provider
                        .install_backend()
                        .await
                        .map_err(|error| AppError::backend_install_failed(backend_name, error))
                },
                Message::OnboardingBackendInstallResult,
            );
        }
        Task::none()
    }

    pub(super) fn handle_onboarding_cancel_install_backend(&mut self) {
        if let AppState::Onboarding(state) = &mut self.state {
            state.confirming_unsafe_install = false;
        }
    }

    pub(super) fn handle_onboarding_backend_install_result(
        &mut self,
        result: Result<(), AppError>,
    ) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.backend_installing = false;
            state.confirming_unsafe_install = false;
            match result {
                Ok(()) => {
                    state.step = OnboardingStep::ConfigureShell;
                }
                Err(error) => {
                    state.install_error = Some(error);
                }
            }
        }
        Task::none()
    }

    pub(super) fn handle_onboarding_configure_shell(
        &mut self,
        shell_type: versi_shell::ShellType,
    ) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            if let Some(shell) = state
                .detected_shells
                .iter_mut()
                .find(|s| s.shell_type == shell_type)
            {
                shell.configuring = true;
                shell.error = None;
            }

            let backend_opts = self.settings.shell_options_for(
                BackendKind::from_name(self.provider.name()).unwrap_or(BackendKind::DEFAULT),
            );
            let options = versi_shell::ShellInitOptions {
                use_on_cd: backend_opts.use_on_cd,
                resolve_engines: backend_opts.resolve_engines,
                corepack_enabled: backend_opts.corepack_enabled,
            };

            let backend = self.provider.clone();
            let backend_marker = backend.shell_config_marker().to_string();
            let backend_label = backend.shell_config_label().to_string();
            let shell_name = shell_type.name();

            return Task::perform(
                async move {
                    use versi_shell::{ShellConfig, get_or_create_config_path};

                    let config_path = get_or_create_config_path(&shell_type)
                        .ok_or_else(|| AppError::shell_config_path_not_found(shell_name))?;

                    let mut config = ShellConfig::load(shell_type, config_path)
                        .map_err(|e| AppError::shell_config_failed(shell_name, "load config", e))?;

                    if config.has_init(&backend_marker) {
                        let edit = config.update_flags(&backend_marker, &options);
                        if edit.has_changes() {
                            config.apply_edit(&edit).map_err(|e| {
                                AppError::shell_config_failed(shell_name, "update config", e)
                            })?;
                        }
                    } else {
                        let init_command = backend
                            .create_manager(&versi_backend::BackendDetection {
                                found: true,
                                path: None,
                                version: None,
                                in_path: true,
                                data_dir: None,
                            })
                            .shell_init_command(shell_type_to_str(&config.shell_type), &options)
                            .ok_or_else(|| AppError::shell_not_supported(shell_name))?;

                        let edit = config.add_init(&init_command, &backend_label);
                        if edit.has_changes() {
                            config.apply_edit(&edit).map_err(|e| {
                                AppError::shell_config_failed(shell_name, "write config", e)
                            })?;
                        }
                    }

                    Ok::<(), AppError>(())
                },
                Message::OnboardingShellConfigResult,
            );
        }
        Task::none()
    }

    pub(super) fn handle_onboarding_shell_config_result(&mut self, result: &Result<(), AppError>) {
        if let AppState::Onboarding(state) = &mut self.state {
            for shell in &mut state.detected_shells {
                if shell.configuring {
                    shell.configuring = false;
                    match result {
                        Ok(()) => {
                            shell.configured = true;
                            shell.error = None;
                        }
                        Err(error) => {
                            shell.error = Some(error.clone());
                        }
                    }
                    break;
                }
            }
        }
    }

    pub(super) fn handle_onboarding_complete(&mut self) -> Task<Message> {
        let all_providers = self.all_providers();
        let preferred = self.settings.preferred_backend;
        Task::perform(
            super::init::initialize(all_providers, preferred),
            |result| Message::Initialized(Box::new(result)),
        )
    }
}

fn shell_type_to_str(shell_type: &versi_shell::ShellType) -> &'static str {
    match shell_type {
        versi_shell::ShellType::Bash => "bash",
        versi_shell::ShellType::Zsh => "zsh",
        versi_shell::ShellType::Fish => "fish",
        versi_shell::ShellType::PowerShell => "powershell",
        versi_shell::ShellType::Cmd => "cmd",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use versi_backend::BackendProvider;

    use super::{Versi, shell_type_to_str};
    use crate::backend_kind::BackendKind;
    use crate::error::{AppError, AppErrorDetail};
    use crate::settings::AppSettings;
    use crate::state::{
        AppState, BackendOption, OnboardingState, OnboardingStep, ShellConfigStatus,
    };

    fn test_onboarding_app(backend_count: usize) -> Versi {
        let fnm_provider: Arc<dyn BackendProvider> = Arc::new(versi_fnm::FnmProvider::new());
        let nvm_provider: Arc<dyn BackendProvider> = Arc::new(versi_nvm::NvmProvider::new());
        let asdf_provider: Arc<dyn BackendProvider> = Arc::new(versi_asdf::AsdfProvider::new());
        let volta_provider: Arc<dyn BackendProvider> = Arc::new(versi_volta::VoltaProvider::new());

        let mut providers: HashMap<BackendKind, Arc<dyn BackendProvider>> = HashMap::new();
        providers.insert(BackendKind::Fnm, fnm_provider.clone());
        providers.insert(BackendKind::Nvm, nvm_provider);
        providers.insert(BackendKind::Asdf, asdf_provider);
        providers.insert(BackendKind::Volta, volta_provider);

        let mut onboarding = OnboardingState::new();
        onboarding.available_backends = if backend_count > 3 {
            vec![
                BackendOption {
                    kind: BackendKind::Fnm,
                    display_name: "fnm",
                    detected: true,
                },
                BackendOption {
                    kind: BackendKind::Nvm,
                    display_name: "nvm",
                    detected: true,
                },
                BackendOption {
                    kind: BackendKind::Asdf,
                    display_name: "asdf",
                    detected: true,
                },
                BackendOption {
                    kind: BackendKind::Volta,
                    display_name: "volta",
                    detected: true,
                },
            ]
        } else if backend_count > 2 {
            vec![
                BackendOption {
                    kind: BackendKind::Fnm,
                    display_name: "fnm",
                    detected: true,
                },
                BackendOption {
                    kind: BackendKind::Nvm,
                    display_name: "nvm",
                    detected: true,
                },
                BackendOption {
                    kind: BackendKind::Asdf,
                    display_name: "asdf",
                    detected: true,
                },
            ]
        } else if backend_count > 1 {
            vec![
                BackendOption {
                    kind: BackendKind::Fnm,
                    display_name: "fnm",
                    detected: true,
                },
                BackendOption {
                    kind: BackendKind::Nvm,
                    display_name: "nvm",
                    detected: true,
                },
                BackendOption {
                    kind: BackendKind::Volta,
                    display_name: "volta",
                    detected: true,
                },
            ]
        } else {
            vec![BackendOption {
                kind: BackendKind::Fnm,
                display_name: "fnm",
                detected: true,
            }]
        };

        Versi {
            state: AppState::Onboarding(onboarding),
            settings: AppSettings::default(),
            window_id: None,
            pending_minimize: false,
            pending_show: false,
            window_visible: true,
            backend_path: PathBuf::from("fnm"),
            backend_in_path: true,
            backend_dir: None,
            window_size: None,
            window_position: None,
            http_client: reqwest::Client::new(),
            providers,
            provider: fnm_provider,
            system_theme_mode: iced::theme::Mode::None,
        }
    }

    #[test]
    fn onboarding_next_from_welcome_uses_backend_count() {
        let mut multi = test_onboarding_app(2);
        let _ = multi.handle_onboarding_next();
        let state = multi.onboarding_state();
        assert_eq!(state.step, OnboardingStep::SelectBackend);

        let mut single = test_onboarding_app(1);
        let _ = single.handle_onboarding_next();
        let state = single.onboarding_state();
        assert_eq!(state.step, OnboardingStep::InstallBackend);
    }

    #[test]
    fn onboarding_back_from_install_uses_backend_count() {
        let mut multi = test_onboarding_app(2);
        if let AppState::Onboarding(state) = &mut multi.state {
            state.step = OnboardingStep::InstallBackend;
        }
        multi.handle_onboarding_back();
        let state = multi.onboarding_state();
        assert_eq!(state.step, OnboardingStep::SelectBackend);

        let mut single = test_onboarding_app(1);
        if let AppState::Onboarding(state) = &mut single.state {
            state.step = OnboardingStep::InstallBackend;
        }
        single.handle_onboarding_back();
        let state = single.onboarding_state();
        assert_eq!(state.step, OnboardingStep::Welcome);
    }

    #[test]
    fn onboarding_install_backend_opens_confirmation_prompt() {
        let mut app = test_onboarding_app(1);
        if let AppState::Onboarding(state) = &mut app.state {
            state.install_error = Some(AppError::backend_install_failed("fnm", "old error"));
        }

        let _ = app.handle_onboarding_install_backend();

        let state = app.onboarding_state();
        assert!(state.confirming_unsafe_install);
        assert!(!state.backend_installing);
        assert!(state.install_error.is_some());
    }

    #[test]
    fn onboarding_cancel_install_backend_closes_confirmation_prompt() {
        let mut app = test_onboarding_app(1);
        if let AppState::Onboarding(state) = &mut app.state {
            state.confirming_unsafe_install = true;
        }

        app.handle_onboarding_cancel_install_backend();

        let state = app.onboarding_state();
        assert!(!state.confirming_unsafe_install);
    }

    #[test]
    fn onboarding_confirm_install_backend_starts_install_and_clears_error() {
        let mut app = test_onboarding_app(1);
        if let AppState::Onboarding(state) = &mut app.state {
            state.confirming_unsafe_install = true;
            state.install_error = Some(AppError::backend_install_failed("fnm", "old error"));
        }

        let _ = app.handle_onboarding_confirm_install_backend();

        let state = app.onboarding_state();
        assert!(state.backend_installing);
        assert!(!state.confirming_unsafe_install);
        assert!(state.install_error.is_none());
    }

    #[test]
    fn onboarding_backend_install_result_updates_flags_and_step() {
        let mut app = test_onboarding_app(1);
        if let AppState::Onboarding(state) = &mut app.state {
            state.backend_installing = true;
            state.confirming_unsafe_install = true;
            state.step = OnboardingStep::InstallBackend;
            state.install_error = Some(AppError::backend_install_failed("fnm", "old error"));
        }

        let _ = app.handle_onboarding_backend_install_result(Ok(()));
        let state = app.onboarding_state();
        assert!(!state.backend_installing);
        assert!(!state.confirming_unsafe_install);
        assert_eq!(state.step, OnboardingStep::ConfigureShell);

        let mut app = test_onboarding_app(1);
        if let AppState::Onboarding(state) = &mut app.state {
            state.backend_installing = true;
            state.confirming_unsafe_install = true;
        }
        let _ = app.handle_onboarding_backend_install_result(Err(
            AppError::backend_install_failed("fnm", "install failed"),
        ));
        let state = app.onboarding_state();
        assert!(!state.backend_installing);
        assert!(!state.confirming_unsafe_install);
        assert_eq!(
            state.install_error,
            Some(AppError::BackendInstallFailed {
                backend: "fnm",
                details: AppErrorDetail::from("install failed")
            })
        );
    }

    #[test]
    fn onboarding_shell_config_result_applies_to_first_configuring_shell() {
        let mut app = test_onboarding_app(1);
        if let AppState::Onboarding(state) = &mut app.state {
            state.detected_shells = vec![
                ShellConfigStatus {
                    shell_type: versi_shell::ShellType::Bash,
                    shell_name: "bash".to_string(),
                    configured: false,
                    config_path: None,
                    configuring: true,
                    error: None,
                },
                ShellConfigStatus {
                    shell_type: versi_shell::ShellType::Zsh,
                    shell_name: "zsh".to_string(),
                    configured: false,
                    config_path: None,
                    configuring: true,
                    error: None,
                },
            ];
        }

        app.handle_onboarding_shell_config_result(&Ok(()));
        let state = app.onboarding_state();
        assert!(state.detected_shells[0].configured);
        assert!(!state.detected_shells[0].configuring);
        assert!(state.detected_shells[0].error.is_none());
        assert!(state.detected_shells[1].configuring);

        let mut app = test_onboarding_app(1);
        if let AppState::Onboarding(state) = &mut app.state {
            state.detected_shells = vec![ShellConfigStatus {
                shell_type: versi_shell::ShellType::Fish,
                shell_name: "fish".to_string(),
                configured: false,
                config_path: None,
                configuring: true,
                error: None,
            }];
        }

        let err = AppError::shell_config_failed("Fish", "write config", "config failed");
        app.handle_onboarding_shell_config_result(&Err(err.clone()));
        let state = app.onboarding_state();
        assert_eq!(state.detected_shells[0].error, Some(err));
        assert!(!state.detected_shells[0].configured);
        assert!(!state.detected_shells[0].configuring);
    }

    #[test]
    fn shell_type_to_str_maps_expected_values() {
        assert_eq!(shell_type_to_str(&versi_shell::ShellType::Bash), "bash");
        assert_eq!(shell_type_to_str(&versi_shell::ShellType::Zsh), "zsh");
        assert_eq!(shell_type_to_str(&versi_shell::ShellType::Fish), "fish");
        assert_eq!(
            shell_type_to_str(&versi_shell::ShellType::PowerShell),
            "powershell"
        );
        assert_eq!(shell_type_to_str(&versi_shell::ShellType::Cmd), "cmd");
    }
}
