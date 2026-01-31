use log::{debug, error, info, trace};

use iced::Task;

use versi_platform::EnvironmentId;

use crate::message::Message;
use crate::state::AppState;

use super::Versi;
use super::init::create_backend_for_environment;

impl Versi {
    pub(super) fn handle_environment_loaded(
        &mut self,
        env_id: EnvironmentId,
        versions: Vec<versi_core::InstalledVersion>,
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
            && let Some(id) = self.window_id
        {
            self.pending_minimize = false;
            return Task::batch([
                Task::done(Message::HideDockIcon),
                iced::window::set_mode(id, iced::window::Mode::Hidden),
            ]);
        }

        Task::none()
    }

    pub(super) fn handle_environment_error(
        &mut self,
        env_id: EnvironmentId,
        error: String,
    ) -> Task<Message> {
        error!("Environment error for {:?}: {}", env_id, error);

        if let AppState::Main(state) = &mut self.state
            && let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id)
        {
            env.loading = false;
            env.error = Some(error);
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

            let load_task = if needs_load {
                info!("Loading versions for environment: {:?}", env_id);
                let env = state.active_environment_mut();
                env.loading = true;

                let backend = state.backend.clone();

                Task::perform(
                    async move {
                        debug!("Fetching installed versions for {:?}...", env_id);
                        let versions = backend.list_installed().await.unwrap_or_default();
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
            return Task::batch([load_task, backend_update_task]);
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

            return Task::perform(
                async move {
                    let versions = backend.list_installed().await.unwrap_or_default();
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
