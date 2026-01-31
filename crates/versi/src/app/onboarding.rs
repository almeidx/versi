use iced::Task;

use crate::message::Message;
use crate::state::{AppState, OnboardingStep};

use super::Versi;

impl Versi {
    pub(super) fn handle_onboarding_next(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
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
            state.step = match state.step {
                OnboardingStep::Welcome => OnboardingStep::Welcome,
                OnboardingStep::SelectBackend => OnboardingStep::Welcome,
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

    pub(super) fn handle_onboarding_select_backend(&mut self, name: String) {
        if let AppState::Onboarding(state) = &mut self.state {
            state.selected_backend = Some(name.clone());
        }
        self.settings.preferred_backend = Some(name.clone());
        let _ = self.settings.save();

        if let Some(provider) = self.providers.get(name.as_str()) {
            self.provider = provider.clone();
        }
    }

    pub(super) fn handle_onboarding_install_backend(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.backend_installing = true;
            state.install_error = None;

            let provider = self.provider.clone();
            return Task::perform(
                async move { provider.install_backend().await.map_err(|e| e.to_string()) },
                Message::OnboardingBackendInstallResult,
            );
        }
        Task::none()
    }

    pub(super) fn handle_onboarding_backend_install_result(
        &mut self,
        result: Result<(), String>,
    ) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.backend_installing = false;
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

            let options = versi_shell::ShellInitOptions {
                use_on_cd: self.settings.shell_options.use_on_cd,
                resolve_engines: self.settings.shell_options.resolve_engines,
                corepack_enabled: self.settings.shell_options.corepack_enabled,
            };

            let backend = self.provider.clone();
            let backend_marker = backend.shell_config_marker().to_string();
            let backend_label = backend.shell_config_label().to_string();

            return Task::perform(
                async move {
                    use versi_shell::{ShellConfig, get_or_create_config_path};

                    let config_path = get_or_create_config_path(&shell_type)
                        .ok_or_else(|| "No config file path found".to_string())?;

                    let mut config =
                        ShellConfig::load(shell_type, config_path).map_err(|e| e.to_string())?;

                    if config.has_init(&backend_marker) {
                        let edit = config.update_flags(&backend_marker, &options);
                        if edit.has_changes() {
                            config.apply_edit(&edit).map_err(|e| e.to_string())?;
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
                            .ok_or_else(|| "Shell not supported".to_string())?;

                        let edit = config.add_init(&init_command, &backend_label);
                        if edit.has_changes() {
                            config.apply_edit(&edit).map_err(|e| e.to_string())?;
                        }
                    }

                    Ok(())
                },
                Message::OnboardingShellConfigResult,
            );
        }
        Task::none()
    }

    pub(super) fn handle_onboarding_shell_config_result(&mut self, result: Result<(), String>) {
        if let AppState::Onboarding(state) = &mut self.state {
            for shell in &mut state.detected_shells {
                if shell.configuring {
                    shell.configuring = false;
                    match &result {
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
        let preferred = self.settings.preferred_backend.clone();
        Task::perform(
            super::init::initialize(all_providers, preferred),
            Message::Initialized,
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
