//! Environment switching, version loading, and search.
//!
//! Handles messages: `EnvironmentSelected`, `EnvironmentLoaded`, `RefreshEnvironment`,
//! `VersionGroupToggled`, `SearchChanged`

use std::time::Duration;

use log::{debug, info, trace};

use iced::Task;
use tokio_util::sync::CancellationToken;

use versi_platform::EnvironmentId;

use crate::error::AppError;
use crate::message::Message;
use crate::state::{AppState, MainState, MainViewKind, SearchFilter};

use super::Versi;
use super::async_helpers::run_with_timeout;
use super::init::create_backend_for_environment;

const BACKGROUND_PRELOAD_DELAY: Duration = Duration::from_millis(1_500);

fn environment_needs_load(env: &crate::state::EnvironmentState) -> bool {
    !env.loaded && env.load_cancel_token.is_none()
}

fn spawn_environment_load(
    env_id: EnvironmentId,
    request_seq: u64,
    cancel_token: CancellationToken,
    backend: std::sync::Arc<dyn versi_backend::VersionManager>,
    fetch_timeout: std::time::Duration,
) -> Task<Message> {
    Task::perform(
        async move {
            let result = tokio::select! {
                () = cancel_token.cancelled() => {
                    Err(AppError::operation_cancelled("Loading versions"))
                }
                result = run_with_timeout(
                    fetch_timeout,
                    "Loading versions",
                    backend.list_installed(),
                    AppError::environment_load_failed,
                ) => result
            };
            (env_id, request_seq, result)
        },
        |(env_id, request_seq, result)| Message::EnvironmentLoaded {
            env_id,
            request_seq,
            result,
        },
    )
}

fn collect_background_preload_targets(
    state: &MainState,
    active_env_id: &EnvironmentId,
) -> Vec<EnvironmentId> {
    state
        .environments
        .iter()
        .filter(|env| env.available)
        .filter(|env| &env.id != active_env_id)
        .filter(|env| !env.loaded)
        .map(|env| env.id.clone())
        .collect()
}

impl Versi {
    pub(super) fn handle_environment_loaded(
        &mut self,
        env_id: &EnvironmentId,
        request_seq: u64,
        result: Result<Vec<versi_backend::InstalledVersion>, AppError>,
    ) -> Task<Message> {
        match &result {
            Ok(versions) => {
                info!(
                    "Environment loaded: {:?} with {} versions",
                    env_id,
                    versions.len()
                );
                for v in versions {
                    trace!(
                        "  Installed version: {} (default={})",
                        v.version, v.is_default
                    );
                }
            }
            Err(error) => {
                info!("Environment load failed for {env_id:?}: {error}");
            }
        }

        if let AppState::Main(state) = &mut self.state
            && let Some(env) = state.environments.iter_mut().find(|e| &e.id == env_id)
        {
            if env.load_request_seq != request_seq {
                debug!(
                    "Ignoring stale environment load for {:?}: request_seq={} current_seq={}",
                    env_id, request_seq, env.load_request_seq
                );
                return Task::none();
            }

            env.load_cancel_token = None;

            match result {
                Ok(versions) => env.update_versions(versions),
                Err(error) => {
                    env.loading = false;
                    env.error = Some(error);
                }
            }

            state.recompute_banner_stats();
        }
        let preload_task = self.schedule_background_preloads_after_active_load(env_id);
        self.update_tray_tooltip();

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
            return Task::batch([preload_task, Task::done(Message::HideDockIcon), hide_task]);
        }

        preload_task
    }

    fn schedule_background_preloads_after_active_load(
        &mut self,
        loaded_env_id: &EnvironmentId,
    ) -> Task<Message> {
        let AppState::Main(state) = &mut self.state else {
            return Task::none();
        };

        if state.background_preload_started {
            return Task::none();
        }

        let active_env_id = state.active_environment().id.clone();
        if &active_env_id != loaded_env_id {
            return Task::none();
        }

        let targets = collect_background_preload_targets(state, &active_env_id);
        state.background_preload_started = true;
        if targets.is_empty() {
            return Task::none();
        }

        Task::batch(targets.into_iter().map(|env_id| {
            Task::perform(
                async move {
                    tokio::time::sleep(BACKGROUND_PRELOAD_DELAY).await;
                    env_id
                },
                |env_id| Message::StartBackgroundEnvironmentPreload { env_id },
            )
        }))
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

            info!("Switching to environment {idx}");
            state.active_environment_idx = idx;
            state.recompute_banner_stats();

            let env = &state.environments[idx];
            let env_id = env.id.clone();
            debug!("Selected environment: {env_id:?}");

            let needs_load = environment_needs_load(env);
            debug!("Environment needs loading: {needs_load}");

            let env_provider = self
                .providers
                .get(&env.backend_name)
                .cloned()
                .unwrap_or_else(|| self.provider.clone());
            self.provider = env_provider.clone();

            let new_backend = create_backend_for_environment(
                &env_id,
                &self.backend_path,
                self.backend_in_path,
                self.backend_dir.as_deref(),
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
                info!("Loading versions for environment: {env_id:?}");
                let (env_id, request_seq, cancel_token) =
                    state.active_environment_mut().prepare_load();
                let backend = state.backend.clone();
                let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);
                spawn_environment_load(env_id, request_seq, cancel_token, backend, fetch_timeout)
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

    pub(super) fn handle_start_background_environment_preload(
        &mut self,
        env_id: &EnvironmentId,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let Some(env_idx) = state.environments.iter().position(|env| &env.id == env_id) else {
                return Task::none();
            };

            let (available, already_loaded, already_loading, backend_name, target_env_id) = {
                let env = &state.environments[env_idx];
                (
                    env.available,
                    env.loaded,
                    env.load_cancel_token.is_some(),
                    env.backend_name,
                    env.id.clone(),
                )
            };

            if !available || already_loaded || already_loading {
                return Task::none();
            }

            let env_provider = self
                .providers
                .get(&backend_name)
                .cloned()
                .unwrap_or_else(|| self.provider.clone());
            let backend = create_backend_for_environment(
                &target_env_id,
                &self.backend_path,
                self.backend_in_path,
                self.backend_dir.as_deref(),
                &env_provider,
            );

            let (env_id, request_seq, cancel_token) = state.environments[env_idx].prepare_load();
            let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);
            return spawn_environment_load(
                env_id,
                request_seq,
                cancel_token,
                backend,
                fetch_timeout,
            );
        }
        Task::none()
    }

    pub(super) fn handle_refresh_environment(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let (env_id, request_seq, cancel_token) = state.active_environment_mut().prepare_load();
            state.refresh_rotation = std::f32::consts::TAU / 40.0;
            let backend = state.backend.clone();
            let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);
            return spawn_environment_load(
                env_id,
                request_seq,
                cancel_token,
                backend,
                fetch_timeout,
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
            if state.keyboard_list_mode
                && is_keyboard_action_input_noise(&state.search_query, &query)
            {
                return;
            }

            if state.keyboard_list_mode {
                state.keyboard_list_mode = false;
            }

            if query.is_empty() {
                state.active_filters.clear();
            }
            state.hovered_version = None;
            state.search_query = query;
        }
    }

    pub(super) fn handle_search_filter_toggled(&mut self, filter: SearchFilter) {
        if let AppState::Main(state) = &mut self.state {
            if state.active_filters.contains(&filter) {
                state.active_filters.remove(&filter);
            } else {
                match filter {
                    SearchFilter::Installed => {
                        state.active_filters.remove(&SearchFilter::NotInstalled);
                    }
                    SearchFilter::NotInstalled => {
                        state.active_filters.remove(&SearchFilter::Installed);
                    }
                    SearchFilter::Eol => {
                        state.active_filters.remove(&SearchFilter::Active);
                    }
                    SearchFilter::Active => {
                        state.active_filters.remove(&SearchFilter::Eol);
                    }
                    SearchFilter::Lts => {}
                }
                state.active_filters.insert(filter);
            }
        }
    }
}

fn is_keyboard_action_input_noise(previous: &str, next: &str) -> bool {
    if !next.starts_with(previous) {
        return false;
    }

    let suffix = &next[previous.len()..];
    if suffix.chars().count() != 1 {
        return false;
    }

    matches!(
        suffix.chars().next(),
        Some('i' | 'I' | 'd' | 'D' | 'u' | 'U')
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tokio_util::sync::CancellationToken;
    use versi_backend::{InstalledVersion, NodeVersion};
    use versi_platform::EnvironmentId;

    use super::super::test_app_with_two_environments;
    use super::*;
    use crate::backend_kind::BackendKind;
    use crate::state::EnvironmentState;

    fn installed(version: &str, is_default: bool) -> InstalledVersion {
        crate::test_fixtures::installed(version, is_default)
    }

    #[test]
    fn search_changed_clears_filters_when_query_becomes_empty() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.active_filters = HashSet::from([SearchFilter::Lts, SearchFilter::Installed]);
        state.search_query = "lts".to_string();

        app.handle_search_changed(String::new());

        let state = app.main_state();
        assert!(state.active_filters.is_empty());
        assert_eq!(state.search_query, "");
    }

    #[test]
    fn search_changed_ignores_action_key_noise_while_keyboard_list_mode_active() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.keyboard_list_mode = true;
        state.search_query = "24".to_string();

        app.handle_search_changed("24i".to_string());

        let state = app.main_state();
        assert_eq!(state.search_query, "24");
        assert!(state.keyboard_list_mode);
    }

    #[test]
    fn search_changed_exits_keyboard_list_mode_for_normal_input() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.keyboard_list_mode = true;
        state.search_query = "24".to_string();

        app.handle_search_changed("241".to_string());

        let state = app.main_state();
        assert_eq!(state.search_query, "241");
        assert!(!state.keyboard_list_mode);
    }

    #[test]
    fn search_filter_toggle_enforces_installed_not_installed_exclusivity() {
        let mut app = test_app_with_two_environments();

        app.handle_search_filter_toggled(SearchFilter::Installed);
        app.handle_search_filter_toggled(SearchFilter::NotInstalled);

        let state = app.main_state();
        assert!(!state.active_filters.contains(&SearchFilter::Installed));
        assert!(state.active_filters.contains(&SearchFilter::NotInstalled));
    }

    #[test]
    fn search_filter_toggle_enforces_eol_active_exclusivity() {
        let mut app = test_app_with_two_environments();

        app.handle_search_filter_toggled(SearchFilter::Active);
        app.handle_search_filter_toggled(SearchFilter::Eol);

        let state = app.main_state();
        assert!(!state.active_filters.contains(&SearchFilter::Active));
        assert!(state.active_filters.contains(&SearchFilter::Eol));
    }

    #[test]
    fn version_group_toggled_flips_target_group_only() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut().active_environment_mut().version_groups = vec![
            versi_backend::VersionGroup {
                major: 22,
                versions: Vec::new(),
                is_expanded: true,
            },
            versi_backend::VersionGroup {
                major: 20,
                versions: Vec::new(),
                is_expanded: false,
            },
        ];

        app.handle_version_group_toggled(20);

        let state = app.main_state();
        let groups = &state.active_environment().version_groups;
        assert!(groups.iter().any(|g| g.major == 20 && g.is_expanded));
        assert!(groups.iter().any(|g| g.major == 22 && g.is_expanded));
    }

    #[test]
    fn refresh_environment_cancels_previous_load_token() {
        let mut app = test_app_with_two_environments();
        let old_token = CancellationToken::new();
        app.main_state_mut()
            .active_environment_mut()
            .load_cancel_token = Some(old_token.clone());

        let _ = app.handle_refresh_environment();

        assert!(old_token.is_cancelled());
        let state = app.main_state();
        assert!(state.active_environment().load_cancel_token.is_some());
    }

    #[test]
    fn collect_background_preload_targets_includes_unloaded_available_non_active() {
        let app = test_app_with_two_environments();
        let state = app.main_state();
        let active_env_id = state.active_environment().id.clone();

        let targets = collect_background_preload_targets(state, &active_env_id);

        assert_eq!(targets.len(), 1);
        assert_eq!(
            targets[0],
            EnvironmentId::Wsl {
                distro: "Ubuntu".to_string(),
                backend_path: "/home/user/.nvm/nvm.sh".to_string()
            }
        );
    }

    #[test]
    fn collect_background_preload_targets_excludes_loaded_and_unavailable() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.environments[1].installed_versions = vec![installed("v20.11.0", true)];
        state.environments[1].loaded = true;
        state.environments.push(EnvironmentState::unavailable(
            EnvironmentId::Wsl {
                distro: "Debian".to_string(),
                backend_path: "/home/user/.nvm/nvm.sh".to_string(),
            },
            BackendKind::Nvm,
            "Not running",
        ));
        let active_env_id = state.active_environment().id.clone();

        let targets = collect_background_preload_targets(state, &active_env_id);

        assert!(targets.is_empty());
    }

    #[test]
    fn environment_loaded_starts_background_preload_once_after_active_load() {
        let mut app = test_app_with_two_environments();
        let non_active_env = EnvironmentId::Wsl {
            distro: "Ubuntu".to_string(),
            backend_path: "/home/user/.nvm/nvm.sh".to_string(),
        };

        let _ = app.handle_environment_loaded(&non_active_env, 0, Ok(vec![]));
        assert!(!app.main_state().background_preload_started);

        let _ = app.handle_environment_loaded(
            &EnvironmentId::Native,
            0,
            Ok(vec![InstalledVersion {
                version: NodeVersion::new(20, 11, 0),
                is_default: true,
                lts_codename: None,
                disk_size: None,
            }]),
        );
        assert!(app.main_state().background_preload_started);
    }

    #[test]
    fn start_background_preload_marks_target_environment_loading() {
        let mut app = test_app_with_two_environments();
        let env_id = EnvironmentId::Wsl {
            distro: "Ubuntu".to_string(),
            backend_path: "/home/user/.nvm/nvm.sh".to_string(),
        };

        let state = app.main_state_mut();
        let target = state
            .environments
            .iter_mut()
            .find(|env| env.id == env_id)
            .expect("expected target environment");
        target.loading = false;
        target.load_request_seq = 5;
        target.load_cancel_token = None;
        target.installed_versions.clear();

        let _ = app.handle_start_background_environment_preload(&env_id);

        let state = app.main_state();
        let target = state
            .environments
            .iter()
            .find(|env| env.id == env_id)
            .expect("expected target environment");
        assert!(target.loading);
        assert_eq!(target.load_request_seq, 6);
        assert!(target.load_cancel_token.is_some());
    }

    #[test]
    fn start_background_preload_skips_loaded_environment() {
        let mut app = test_app_with_two_environments();
        let env_id = EnvironmentId::Wsl {
            distro: "Ubuntu".to_string(),
            backend_path: "/home/user/.nvm/nvm.sh".to_string(),
        };

        let state = app.main_state_mut();
        let target = state
            .environments
            .iter_mut()
            .find(|env| env.id == env_id)
            .expect("expected target environment");
        target.loading = false;
        target.load_request_seq = 3;
        target.load_cancel_token = None;
        target.installed_versions = vec![installed("v20.11.0", true)];
        target.loaded = true;

        let _ = app.handle_start_background_environment_preload(&env_id);

        let state = app.main_state();
        let target = state
            .environments
            .iter()
            .find(|env| env.id == env_id)
            .expect("expected target environment");
        assert_eq!(target.load_request_seq, 3);
        assert!(target.load_cancel_token.is_none());
    }

    #[test]
    fn selecting_environment_with_inflight_preload_does_not_restart_load() {
        let mut app = test_app_with_two_environments();
        let old_token = CancellationToken::new();
        let state = app.main_state_mut();
        state.environments[1].loading = true;
        state.environments[1].installed_versions.clear();
        state.environments[1].load_request_seq = 7;
        state.environments[1].load_cancel_token = Some(old_token.clone());

        let _ = app.handle_environment_selected(1);

        let state = app.main_state();
        assert_eq!(state.active_environment_idx, 1);
        assert_eq!(state.active_environment().load_request_seq, 7);
        assert!(!old_token.is_cancelled());
        assert!(state.active_environment().load_cancel_token.is_some());
    }
}
