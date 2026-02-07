use std::time::Duration;

use log::{debug, info, trace};

use iced::Task;

use versi_platform::EnvironmentId;

use crate::message::Message;
use crate::state::{AppState, MainViewKind};

use super::Versi;
use super::init::create_backend_for_environment;

impl Versi {
    pub(super) fn handle_environment_loaded(
        &mut self,
        env_id: EnvironmentId,
        versions: Vec<versi_backend::InstalledVersion>,
    ) -> Task<Message> {
        info!(
            "Environment loaded: {:?} with {} versions",
            env_id,
            versions.len()
        );
        for v in &versions {
            trace!(
                "  Installed version: {} (default={})",
                v.version, v.is_default
            );
        }

        if let AppState::Main(state) = &mut self.state
            && let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id)
        {
            env.update_versions(versions);
        }
        self.update_tray_menu();

        if self.pending_minimize
            && !self.pending_show
            && let Some(id) = self.window_id
        {
            self.pending_minimize = false;
            let hide_task = if super::platform::is_wayland() {
                iced::window::minimize(id, true)
            } else {
                iced::window::set_mode(id, iced::window::Mode::Hidden)
            };
            return Task::batch([Task::done(Message::HideDockIcon), hide_task]);
        }

        Task::none()
    }

    pub(super) fn handle_environment_selected(&mut self, idx: usize) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if idx >= state.environments.len() || idx == state.active_environment_idx {
                debug!(
                    "Environment selection ignored: idx={}, current={}",
                    idx, state.active_environment_idx
                );
                return Task::none();
            }

            info!("Switching to environment {}", idx);
            state.active_environment_idx = idx;

            let env = &state.environments[idx];
            let env_id = env.id.clone();
            debug!("Selected environment: {:?}", env_id);

            let needs_load =
                env.loading || (env.installed_versions.is_empty() && env.error.is_none());
            debug!("Environment needs loading: {}", needs_load);

            let env_provider = self
                .providers
                .get(env.backend_name)
                .cloned()
                .unwrap_or_else(|| self.provider.clone());

            let new_backend = create_backend_for_environment(
                &env_id,
                &self.backend_path,
                &self.backend_dir,
                &env_provider,
            );
            state.backend = new_backend;
            state.backend_name = env.backend_name;

            state.backend_update = None;

            let in_settings = state.view == MainViewKind::Settings;
            if in_settings {
                state.settings_state.checking_shells = true;
            }

            let load_task = if needs_load {
                info!("Loading versions for environment: {:?}", env_id);
                let env = state.active_environment_mut();
                env.loading = true;

                let backend = state.backend.clone();
                let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);

                Task::perform(
                    async move {
                        debug!("Fetching installed versions for {:?}...", env_id);
                        let versions =
                            tokio::time::timeout(fetch_timeout, backend.list_installed())
                                .await
                                .unwrap_or(Ok(Vec::new()))
                                .unwrap_or_default();
                        debug!(
                            "Environment {:?} loaded: {} versions",
                            env_id,
                            versions.len(),
                        );
                        (env_id, versions)
                    },
                    |(env_id, versions)| Message::EnvironmentLoaded { env_id, versions },
                )
            } else {
                Task::none()
            };

            let backend_update_task = self.handle_check_for_backend_update();
            let shell_task = if in_settings {
                self.handle_check_shell_setup()
            } else {
                Task::none()
            };

            return Task::batch([load_task, backend_update_task, shell_task]);
        }
        Task::none()
    }

    pub(super) fn handle_refresh_environment(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment_mut();
            env.loading = true;
            env.error = None;
            let env_id = env.id.clone();

            state.refresh_rotation = std::f32::consts::TAU / 40.0;
            let backend = state.backend.clone();
            let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);

            return Task::perform(
                async move {
                    let versions = tokio::time::timeout(fetch_timeout, backend.list_installed())
                        .await
                        .unwrap_or(Ok(Vec::new()))
                        .unwrap_or_default();
                    (env_id, versions)
                },
                |(env_id, versions)| Message::EnvironmentLoaded { env_id, versions },
            );
        }
        Task::none()
    }

    pub(super) fn handle_version_group_toggled(&mut self, major: u32) {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment_mut();
            if let Some(group) = env.version_groups.iter_mut().find(|g| g.major == major) {
                group.is_expanded = !group.is_expanded;
            }
        }
    }

    pub(super) fn handle_search_changed(&mut self, query: String) {
        if let AppState::Main(state) = &mut self.state {
            state.search_query = query;
        }
    }
}
