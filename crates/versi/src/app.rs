use log::{debug, error, info, trace};
use std::path::{Path, PathBuf};
use std::time::Instant;

use iced::{Element, Subscription, Task, Theme};

#[cfg(target_os = "macos")]
pub fn set_dock_visible(visible: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let policy = if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    app.setActivationPolicy(policy);
}

#[cfg(not(target_os = "macos"))]
pub fn set_dock_visible(_visible: bool) {}

#[cfg(windows)]
use versi_core::HideWindow;
use versi_core::{
    FnmBackend, VersionManager, check_for_fnm_update, check_for_update, detect_fnm,
    fetch_release_schedule,
};
use versi_platform::EnvironmentId;
use versi_shell::detect_shells;

use crate::message::{InitResult, Message};
use crate::settings::{AppSettings, ThemeSetting, TrayBehavior};
use crate::state::{
    AppState, EnvironmentState, MainState, MainViewKind, Modal, OnboardingState, OnboardingStep,
    Operation, OperationRequest, QueuedOperation, ShellConfigStatus, ShellSetupStatus,
    ShellVerificationStatus, Toast, UndoAction,
};
use crate::theme::{dark_theme, get_system_theme, light_theme};
use crate::tray::{self, TrayMenuData, TrayMessage};
use crate::views;

pub struct FnmUi {
    state: AppState,
    settings: AppSettings,
    window_id: Option<iced::window::Id>,
    pending_minimize: bool,
    fnm_path: PathBuf,
    fnm_dir: Option<PathBuf>,
    window_size: Option<iced::Size>,
    window_position: Option<iced::Point>,
    http_client: reqwest::Client,
}

impl FnmUi {
    pub fn new() -> (Self, Task<Message>) {
        let settings = AppSettings::load();

        let should_minimize =
            settings.start_minimized && settings.tray_behavior != TrayBehavior::Disabled;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(format!("versi/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();

        let app = Self {
            state: AppState::Loading,
            settings,
            window_id: None,
            pending_minimize: should_minimize,
            fnm_path: PathBuf::from("fnm"),
            fnm_dir: None,
            window_size: None,
            window_position: None,
            http_client,
        };

        let init_task = Task::perform(initialize(), Message::Initialized);

        (app, init_task)
    }

    pub fn title(&self) -> String {
        match &self.state {
            AppState::Loading => "Versi".to_string(),
            AppState::Onboarding(_) => "Versi - Setup".to_string(),
            AppState::Main(state) => {
                if let Some(v) = &state.active_environment().default_version {
                    format!("Versi - Node {}", v)
                } else {
                    "Versi".to_string()
                }
            }
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Initialized(result) => self.handle_initialized(result),
            Message::EnvironmentLoaded {
                env_id,
                versions,
                default_version,
            } => self.handle_environment_loaded(env_id, versions, default_version),
            Message::EnvironmentError { env_id, error } => {
                self.handle_environment_error(env_id, error)
            }
            Message::RefreshEnvironment => self.handle_refresh_environment(),
            Message::VersionGroupToggled { major } => {
                self.handle_version_group_toggled(major);
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.handle_search_changed(query);
                Task::none()
            }
            Message::FetchRemoteVersions => self.handle_fetch_remote_versions(),
            Message::RemoteVersionsFetched(result) => {
                self.handle_remote_versions_fetched(result);
                Task::none()
            }
            Message::ReleaseScheduleFetched(result) => {
                self.handle_release_schedule_fetched(result);
                Task::none()
            }
            Message::CloseModal => {
                if let AppState::Main(state) = &mut self.state {
                    if state.modal.is_some() {
                        state.modal = None;
                    } else if state.view == MainViewKind::About
                        || state.view == MainViewKind::Settings
                    {
                        state.view = MainViewKind::Versions;
                    }
                }
                Task::none()
            }
            Message::OpenChangelog(version) => {
                let url = format!("https://nodejs.org/en/blog/release/{}", version);
                Task::perform(
                    async move {
                        let _ = open::that(&url);
                    },
                    |_| Message::NoOp,
                )
            }
            Message::StartInstall(version) => self.handle_start_install(version),
            Message::InstallProgress { version, progress } => {
                self.handle_install_progress(version, progress);
                Task::none()
            }
            Message::InstallComplete {
                version,
                success,
                error,
            } => self.handle_install_complete(version, success, error),
            Message::RequestUninstall(version) => {
                self.handle_request_uninstall(version);
                Task::none()
            }
            Message::ConfirmUninstall(version) => self.handle_confirm_uninstall(version),
            Message::CancelUninstall => {
                self.handle_close_modal();
                Task::none()
            }
            Message::CancelQueuedOperation(id) => self.handle_cancel_queued_operation(id),
            Message::UninstallComplete {
                version,
                success,
                error,
            } => self.handle_uninstall_complete(version, success, error),
            Message::RequestBulkUpdateMajors => self.handle_request_bulk_update_majors(),
            Message::RequestBulkUninstallEOL => self.handle_request_bulk_uninstall_eol(),
            Message::RequestBulkUninstallMajor { major } => {
                self.handle_request_bulk_uninstall_major(major)
            }
            Message::ConfirmBulkUpdateMajors => self.handle_confirm_bulk_update_majors(),
            Message::ConfirmBulkUninstallEOL => self.handle_confirm_bulk_uninstall_eol(),
            Message::ConfirmBulkUninstallMajor { major } => {
                self.handle_confirm_bulk_uninstall_major(major)
            }
            Message::RequestBulkUninstallMajorExceptLatest { major } => {
                self.handle_request_bulk_uninstall_major_except_latest(major)
            }
            Message::ConfirmBulkUninstallMajorExceptLatest { major } => {
                self.handle_confirm_bulk_uninstall_major_except_latest(major)
            }
            Message::CancelBulkOperation => {
                self.handle_close_modal();
                Task::none()
            }
            Message::SetDefault(version) => self.handle_set_default(version),
            Message::DefaultChanged {
                version,
                previous,
                success,
                error,
            } => self.handle_default_changed(version, previous, success, error),
            Message::ToastDismiss(id) => {
                if let AppState::Main(state) = &mut self.state {
                    state.remove_toast(id);
                }
                Task::none()
            }
            Message::ToastUndo(id) => self.handle_toast_undo(id),
            Message::NavigateToSettings => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Settings;
                    state.settings_state.checking_shells = true;
                }
                let shell_task = self.handle_check_shell_setup();
                let log_stats_task = Task::perform(
                    async {
                        let log_path = versi_platform::AppPaths::new().log_file();
                        std::fs::metadata(&log_path).ok().map(|m| m.len())
                    },
                    Message::LogFileStatsLoaded,
                );
                Task::batch([shell_task, log_stats_task])
            }
            Message::NavigateToAbout => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::About;
                }
                Task::none()
            }
            Message::NavigateToVersions => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Versions;
                }
                Task::none()
            }
            Message::VersionRowHovered(version) => {
                if let AppState::Main(state) = &mut self.state {
                    if state.modal.is_some() {
                        state.hovered_version = None;
                    } else {
                        state.hovered_version = version;
                    }
                }
                Task::none()
            }
            Message::ThemeChanged(theme) => {
                self.settings.theme = theme;
                let _ = self.settings.save();
                Task::none()
            }
            Message::ShellOptionUseOnCdToggled(value) => {
                self.settings.shell_options.use_on_cd = value;
                let _ = self.settings.save();
                self.update_shell_flags()
            }
            Message::ShellOptionResolveEnginesToggled(value) => {
                self.settings.shell_options.resolve_engines = value;
                let _ = self.settings.save();
                self.update_shell_flags()
            }
            Message::ShellOptionCorepackEnabledToggled(value) => {
                self.settings.shell_options.corepack_enabled = value;
                let _ = self.settings.save();
                self.update_shell_flags()
            }
            Message::DebugLoggingToggled(value) => {
                self.settings.debug_logging = value;
                let _ = self.settings.save();
                crate::logging::set_logging_enabled(value);
                if value {
                    info!("Debug logging enabled");
                }
                Task::none()
            }
            Message::CopyToClipboard(text) => iced::clipboard::write(text),
            Message::ClearLogFile => {
                let log_path = versi_platform::AppPaths::new().log_file();
                Task::perform(
                    async move {
                        if log_path.exists() {
                            let _ = std::fs::write(&log_path, "");
                        }
                    },
                    |_| Message::LogFileCleared,
                )
            }
            Message::LogFileCleared => {
                if let AppState::Main(state) = &mut self.state {
                    state.settings_state.log_file_size = Some(0);
                }
                Task::none()
            }
            Message::RevealLogFile => {
                let log_path = versi_platform::AppPaths::new().log_file();
                Task::perform(async move { reveal_in_file_manager(&log_path) }, |_| {
                    Message::NoOp
                })
            }
            Message::LogFileStatsLoaded(size) => {
                if let AppState::Main(state) = &mut self.state {
                    state.settings_state.log_file_size = size;
                }
                Task::none()
            }
            Message::ShellFlagsUpdated(_) => Task::none(),
            Message::CheckShellSetup => self.handle_check_shell_setup(),
            Message::ShellSetupChecked(results) => {
                self.handle_shell_setup_checked(results);
                Task::none()
            }
            Message::ConfigureShell(shell_type) => self.handle_configure_shell(shell_type),
            Message::ShellConfigured(shell_type, result) => {
                self.handle_shell_configured(shell_type, result);
                Task::none()
            }
            Message::OnboardingNext => self.handle_onboarding_next(),
            Message::OnboardingBack => {
                self.handle_onboarding_back();
                Task::none()
            }
            Message::OnboardingInstallFnm => self.handle_onboarding_install_fnm(),
            Message::OnboardingFnmInstallResult(result) => {
                self.handle_onboarding_fnm_install_result(result)
            }
            Message::OnboardingConfigureShell(shell_type) => {
                self.handle_onboarding_configure_shell(shell_type)
            }
            Message::OnboardingShellConfigResult(result) => {
                self.handle_onboarding_shell_config_result(result);
                Task::none()
            }
            Message::OnboardingComplete => self.handle_onboarding_complete(),
            Message::Tick => {
                if let AppState::Main(state) = &mut self.state {
                    state.toasts.retain(|t| !t.is_expired());
                }
                Task::none()
            }
            Message::WindowEvent(iced::window::Event::CloseRequested) | Message::CloseWindow => {
                self.save_window_geometry();
                if self.settings.tray_behavior == TrayBehavior::AlwaysRunning
                    && tray::is_tray_active()
                {
                    if let Some(id) = self.window_id {
                        set_dock_visible(false);
                        iced::window::set_mode(id, iced::window::Mode::Hidden)
                    } else {
                        Task::none()
                    }
                } else {
                    iced::exit()
                }
            }
            Message::WindowEvent(iced::window::Event::Resized(size)) => {
                self.window_size = Some(size);
                Task::none()
            }
            Message::WindowEvent(iced::window::Event::Moved(point)) => {
                self.window_position = Some(point);
                Task::none()
            }
            Message::WindowOpened(id) => {
                self.window_id = Some(id);
                if self.pending_minimize {
                    self.pending_minimize = false;
                    Task::batch([
                        Task::done(Message::HideDockIcon),
                        iced::window::set_mode(id, iced::window::Mode::Hidden),
                    ])
                } else {
                    Task::none()
                }
            }
            Message::HideDockIcon => {
                set_dock_visible(false);
                Task::none()
            }
            Message::WindowEvent(_) => Task::none(),
            Message::CheckForAppUpdate => self.handle_check_for_app_update(),
            Message::AppUpdateChecked(result) => {
                self.handle_app_update_checked(result);
                Task::none()
            }
            Message::OpenAppUpdate => {
                if let AppState::Main(state) = &self.state
                    && let Some(update) = &state.app_update
                {
                    let url = update.release_url.clone();
                    return Task::perform(
                        async move {
                            let _ = open::that(&url);
                        },
                        |_| Message::NoOp,
                    );
                }
                Task::none()
            }
            Message::DismissAppUpdate => {
                if let AppState::Main(state) = &mut self.state {
                    state.app_update = None;
                }
                Task::none()
            }
            Message::CheckForFnmUpdate => self.handle_check_for_fnm_update(),
            Message::FnmUpdateChecked(result) => {
                self.handle_fnm_update_checked(result);
                Task::none()
            }
            Message::FetchReleaseSchedule => self.handle_fetch_release_schedule(),
            Message::OpenFnmUpdate => {
                if let AppState::Main(state) = &self.state
                    && let Some(update) = &state.fnm_update
                {
                    let url = update.release_url.clone();
                    return Task::perform(
                        async move {
                            let _ = open::that(&url);
                        },
                        |_| Message::NoOp,
                    );
                }
                Task::none()
            }
            Message::WindowGeometrySaved => Task::none(),
            Message::OpenLink(url) => Task::perform(
                async move {
                    let _ = open::that(&url);
                },
                |_| Message::NoOp,
            ),
            Message::EnvironmentSelected(idx) => self.handle_environment_selected(idx),
            Message::TrayEvent(tray_msg) => self.handle_tray_event(tray_msg),
            Message::TrayBehaviorChanged(behavior) => self.handle_tray_behavior_changed(behavior),
            Message::StartMinimizedToggled(value) => {
                self.settings.start_minimized = value;
                let _ = self.settings.save();
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            AppState::Loading => views::loading::view(),
            AppState::Onboarding(state) => views::onboarding::view(state),
            AppState::Main(state) => match state.view {
                MainViewKind::Versions => views::main_view::view(state, &self.settings),
                MainViewKind::Settings => {
                    views::settings_view::view(&state.settings_state, &self.settings, state)
                }
                MainViewKind::About => views::about_view::view(),
            },
        }
    }

    pub fn theme(&self) -> Theme {
        match self.settings.theme {
            ThemeSetting::System => get_system_theme(),
            ThemeSetting::Light => light_theme(),
            ThemeSetting::Dark => dark_theme(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick);

        let keyboard = iced::event::listen_with(|event, _status, _id| {
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key, modifiers, ..
            }) = event
            {
                if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
                    return Some(Message::CloseModal);
                }

                #[cfg(target_os = "macos")]
                let close_modifier = modifiers.command();
                #[cfg(not(target_os = "macos"))]
                let close_modifier = modifiers.control();

                if close_modifier
                    && let iced::keyboard::Key::Character(c) = key
                    && c.as_str() == "w"
                {
                    return Some(Message::CloseWindow);
                }

                None
            } else {
                None
            }
        });

        let window_events = iced::event::listen_with(|event, _status, _id| {
            if let iced::Event::Window(window_event) = event {
                Some(Message::WindowEvent(window_event))
            } else {
                None
            }
        });

        let tray_sub = if self.settings.tray_behavior != TrayBehavior::Disabled {
            tray::tray_subscription()
        } else {
            Subscription::none()
        };

        let window_open_sub = iced::window::open_events().map(Message::WindowOpened);

        Subscription::batch([tick, keyboard, window_events, tray_sub, window_open_sub])
    }

    fn save_window_geometry(&mut self) {
        if let (Some(size), Some(pos)) = (self.window_size, self.window_position) {
            self.settings.window_geometry = Some(crate::settings::WindowGeometry {
                width: size.width,
                height: size.height,
                x: pos.x as i32,
                y: pos.y as i32,
            });
            let _ = self.settings.save();
        }
    }

    fn handle_initialized(&mut self, result: InitResult) -> Task<Message> {
        info!(
            "Handling initialization result: fnm_found={}, environments={}",
            result.fnm_found,
            result.environments.len()
        );

        if !result.fnm_found {
            info!("fnm not found, entering onboarding flow");
            let shells = detect_shells();
            debug!("Detected {} shells for configuration", shells.len());

            let shell_statuses: Vec<ShellConfigStatus> = shells
                .into_iter()
                .map(|s| ShellConfigStatus {
                    shell_type: s.shell_type.clone(),
                    shell_name: s.shell_type.name().to_string(),
                    configured: s.is_configured,
                    config_path: s.config_file,
                    configuring: false,
                    error: None,
                })
                .collect();

            let mut onboarding = OnboardingState::new();
            onboarding.detected_shells = shell_statuses;
            self.state = AppState::Onboarding(onboarding);
            return Task::none();
        }

        let fnm_path = result.fnm_path.unwrap_or_else(|| PathBuf::from("fnm"));
        let fnm_dir = result.fnm_dir;

        self.fnm_path = fnm_path.clone();
        self.fnm_dir = fnm_dir.clone();

        let backend = FnmBackend::new(
            fnm_path.clone(),
            result.fnm_version.clone(),
            fnm_dir.clone(),
        );
        let backend = if let Some(dir) = fnm_dir.clone() {
            backend.with_fnm_dir(dir)
        } else {
            backend
        };
        let backend: Box<dyn VersionManager> = Box::new(backend.clone());

        let environments: Vec<EnvironmentState> = result
            .environments
            .iter()
            .map(|env_info| {
                if env_info.available {
                    EnvironmentState::new(env_info.id.clone(), env_info.fnm_version.clone())
                } else {
                    EnvironmentState::unavailable(
                        env_info.id.clone(),
                        env_info
                            .unavailable_reason
                            .as_deref()
                            .unwrap_or("Unavailable"),
                    )
                }
            })
            .collect();

        let mut main_state = MainState::new_with_environments(backend, environments);

        if let Some(disk_cache) = crate::cache::DiskCache::load() {
            debug!(
                "Loaded disk cache from {:?} ({} versions, schedule={})",
                disk_cache.cached_at,
                disk_cache.remote_versions.len(),
                disk_cache.release_schedule.is_some()
            );
            if !disk_cache.remote_versions.is_empty() {
                main_state.available_versions.versions = disk_cache.remote_versions;
                main_state.available_versions.loaded_from_disk = true;
            }
            if let Some(schedule) = disk_cache.release_schedule {
                main_state.available_versions.schedule = Some(schedule);
            }
        }

        self.state = AppState::Main(main_state);

        let mut load_tasks: Vec<Task<Message>> = Vec::new();

        for env_info in &result.environments {
            if !env_info.available {
                debug!(
                    "Skipping load for unavailable environment: {:?}",
                    env_info.id
                );
                continue;
            }

            let env_id = env_info.id.clone();
            let backend = create_backend_for_environment(&env_id, &fnm_path, &fnm_dir);

            load_tasks.push(Task::perform(
                async move {
                    let versions = backend.list_installed().await.unwrap_or_default();
                    let default = backend.default_version().await.ok().flatten();
                    (env_id, versions, default)
                },
                move |(env_id, versions, default)| Message::EnvironmentLoaded {
                    env_id,
                    versions,
                    default_version: default,
                },
            ));
        }

        let fetch_remote = self.handle_fetch_remote_versions();
        let fetch_schedule = self.handle_fetch_release_schedule();
        let check_app_update = self.handle_check_for_app_update();
        let check_fnm_update = self.handle_check_for_fnm_update();

        load_tasks.extend([
            fetch_remote,
            fetch_schedule,
            check_app_update,
            check_fnm_update,
        ]);

        Task::batch(load_tasks)
    }

    fn handle_environment_loaded(
        &mut self,
        env_id: EnvironmentId,
        versions: Vec<versi_core::InstalledVersion>,
        _default_version: Option<versi_core::NodeVersion>,
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

    fn handle_environment_error(&mut self, env_id: EnvironmentId, error: String) -> Task<Message> {
        error!("Environment error for {:?}: {}", env_id, error);

        if let AppState::Main(state) = &mut self.state
            && let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id)
        {
            env.loading = false;
            env.error = Some(error);
        }
        Task::none()
    }

    fn handle_environment_selected(&mut self, idx: usize) -> Task<Message> {
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

            let new_backend =
                create_backend_for_environment(&env_id, &self.fnm_path, &self.fnm_dir);
            state.backend = new_backend;

            state.fnm_update = None;

            let load_task = if needs_load {
                info!("Loading versions for environment: {:?}", env_id);
                let env = state.active_environment_mut();
                env.loading = true;

                let backend = state.backend.clone();

                Task::perform(
                    async move {
                        debug!("Fetching installed versions for {:?}...", env_id);
                        let versions = backend.list_installed().await.unwrap_or_default();
                        debug!("Fetching default version for {:?}...", env_id);
                        let default = backend.default_version().await.ok().flatten();
                        debug!(
                            "Environment {:?} loaded: {} versions, default={:?}",
                            env_id,
                            versions.len(),
                            default
                        );
                        (env_id, versions, default)
                    },
                    |(env_id, versions, default)| Message::EnvironmentLoaded {
                        env_id,
                        versions,
                        default_version: default,
                    },
                )
            } else {
                Task::none()
            };

            let fnm_update_task = self.handle_check_for_fnm_update();
            return Task::batch([load_task, fnm_update_task]);
        }
        Task::none()
    }

    fn handle_refresh_environment(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment_mut();
            env.loading = true;
            env.error = None;

            let env_id = env.id.clone();
            let backend = state.backend.clone();

            return Task::perform(
                async move {
                    let versions = backend.list_installed().await.unwrap_or_default();
                    let default = backend.default_version().await.ok().flatten();
                    (env_id, versions, default)
                },
                |(env_id, versions, default)| Message::EnvironmentLoaded {
                    env_id,
                    versions,
                    default_version: default,
                },
            );
        }
        Task::none()
    }

    fn handle_version_group_toggled(&mut self, major: u32) {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment_mut();
            if let Some(group) = env.version_groups.iter_mut().find(|g| g.major == major) {
                group.is_expanded = !group.is_expanded;
            }
        }
    }

    fn handle_search_changed(&mut self, query: String) {
        if let AppState::Main(state) = &mut self.state {
            state.search_query = query;
        }
    }

    fn handle_fetch_remote_versions(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.available_versions.loading {
                return Task::none();
            }
            state.available_versions.loading = true;

            let backend = state.backend.clone();

            return Task::perform(
                async move {
                    let delays = [0, 2, 5, 15];
                    let mut last_err = String::new();
                    for (attempt, &delay) in delays.iter().enumerate() {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        }
                        match backend.list_remote().await {
                            Ok(versions) => return Ok(versions),
                            Err(e) => {
                                last_err = e.to_string();
                                debug!(
                                    "Remote versions fetch attempt {} failed: {}",
                                    attempt + 1,
                                    last_err
                                );
                            }
                        }
                    }
                    Err(last_err)
                },
                Message::RemoteVersionsFetched,
            );
        }
        Task::none()
    }

    fn handle_remote_versions_fetched(
        &mut self,
        result: Result<Vec<versi_core::RemoteVersion>, String>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            state.available_versions.loading = false;
            match result {
                Ok(versions) => {
                    state.available_versions.versions = versions.clone();
                    state.available_versions.fetched_at = Some(Instant::now());
                    state.available_versions.error = None;
                    state.available_versions.loaded_from_disk = false;

                    let schedule = state.available_versions.schedule.clone();
                    std::thread::spawn(move || {
                        let cache = crate::cache::DiskCache {
                            remote_versions: versions,
                            release_schedule: schedule,
                            cached_at: chrono::Utc::now(),
                        };
                        cache.save();
                    });
                }
                Err(error) => {
                    state.available_versions.error = Some(error);
                }
            }
        }
    }

    fn handle_fetch_release_schedule(&mut self) -> Task<Message> {
        if let AppState::Main(_) = &self.state {
            let client = self.http_client.clone();

            return Task::perform(
                async move {
                    let delays = [0, 2, 5, 15];
                    let mut last_err = String::new();
                    for (attempt, &delay) in delays.iter().enumerate() {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        }
                        match fetch_release_schedule(&client).await {
                            Ok(schedule) => return Ok(schedule),
                            Err(e) => {
                                last_err = e;
                                debug!(
                                    "Release schedule fetch attempt {} failed: {}",
                                    attempt + 1,
                                    last_err
                                );
                            }
                        }
                    }
                    Err(last_err)
                },
                Message::ReleaseScheduleFetched,
            );
        }
        Task::none()
    }

    fn handle_release_schedule_fetched(
        &mut self,
        result: Result<versi_core::ReleaseSchedule, String>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            match result {
                Ok(schedule) => {
                    state.available_versions.schedule = Some(schedule.clone());
                    state.available_versions.schedule_error = None;

                    let versions = state.available_versions.versions.clone();
                    std::thread::spawn(move || {
                        let cache = crate::cache::DiskCache {
                            remote_versions: versions,
                            release_schedule: Some(schedule),
                            cached_at: chrono::Utc::now(),
                        };
                        cache.save();
                    });
                }
                Err(error) => {
                    debug!("Release schedule fetch failed: {}", error);
                    state.available_versions.schedule_error = Some(error);
                }
            }
        }
    }

    fn handle_close_modal(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;
        }
    }

    fn handle_start_install(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;

            if state
                .operation_queue
                .active_installs
                .iter()
                .any(|op| matches!(op, Operation::Install { version: v, .. } if v == &version))
                || state.operation_queue.has_pending_for_version(&version)
            {
                return Task::none();
            }

            if state.operation_queue.is_busy_for_install() {
                let id = state.operation_queue.next_id();
                state.operation_queue.pending.push_back(QueuedOperation {
                    id,
                    request: OperationRequest::Install {
                        version: version.clone(),
                    },
                    queued_at: Instant::now(),
                });
                return Task::none();
            }

            return self.start_install_internal(version);
        }
        Task::none()
    }

    fn start_install_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state
                .operation_queue
                .active_installs
                .push(Operation::Install {
                    version: version.clone(),
                    progress: Default::default(),
                });

            let backend = state.backend.clone();
            let version_clone = version.clone();

            let install_stream = async_stream::stream! {
                match backend.install_with_progress(&version_clone).await {
                    Ok(mut rx) => {
                        let mut final_success = false;
                        let mut last_error: Option<String> = None;
                        while let Some(progress) = rx.recv().await {
                            let is_complete = progress.phase == versi_core::InstallPhase::Complete;
                            let is_failed = progress.phase == versi_core::InstallPhase::Failed;

                            if is_failed {
                                last_error = progress.error.clone();
                            }

                            yield Message::InstallProgress {
                                version: version_clone.clone(),
                                progress,
                            };

                            if is_complete {
                                final_success = true;
                                break;
                            }
                            if is_failed {
                                break;
                            }
                        }
                        yield Message::InstallComplete {
                            version: version_clone.clone(),
                            success: final_success,
                            error: if final_success { None } else { last_error.or_else(|| Some("Installation failed".to_string())) },
                        };
                    }
                    Err(e) => {
                        yield Message::InstallComplete {
                            version: version_clone.clone(),
                            success: false,
                            error: Some(e.to_string()),
                        };
                    }
                }
            };
            return Task::run(install_stream, |msg| msg);
        }
        Task::none()
    }

    fn handle_install_progress(&mut self, version: String, progress: versi_core::InstallProgress) {
        if let AppState::Main(state) = &mut self.state {
            state
                .operation_queue
                .update_install_progress(&version, progress);
        }
    }

    fn handle_install_complete(
        &mut self,
        version: String,
        success: bool,
        error: Option<String>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.remove_completed_install(&version);

            let toast_id = state.next_toast_id();
            if success {
                state.add_toast(Toast::success(
                    toast_id,
                    format!("Node {} installed successfully", version),
                ));
            } else {
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to install Node {}: {}",
                        version,
                        error.unwrap_or_default()
                    ),
                ));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    fn handle_request_uninstall(&mut self, version: String) {
        if let AppState::Main(state) = &mut self.state {
            let is_default = state
                .active_environment()
                .default_version
                .as_ref()
                .is_some_and(|d| d.to_string() == version);
            state.modal = Some(Modal::ConfirmUninstall {
                version,
                is_default,
            });
        }
    }

    fn handle_confirm_uninstall(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;

            if state.operation_queue.is_busy_for_exclusive() {
                let id = state.operation_queue.next_id();
                state.operation_queue.pending.push_back(QueuedOperation {
                    id,
                    request: OperationRequest::Uninstall {
                        version: version.clone(),
                    },
                    queued_at: Instant::now(),
                });
                return Task::none();
            }

            return self.start_uninstall_internal(version);
        }
        Task::none()
    }

    fn start_uninstall_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.exclusive_op = Some(Operation::Uninstall {
                version: version.clone(),
            });

            let backend = state.backend.clone();
            let version_clone = version.clone();

            return Task::perform(
                async move {
                    match backend.uninstall(&version_clone).await {
                        Ok(()) => (version_clone, true, None),
                        Err(e) => (version_clone, false, Some(e.to_string())),
                    }
                },
                |(version, success, error)| Message::UninstallComplete {
                    version,
                    success,
                    error,
                },
            );
        }
        Task::none()
    }

    fn handle_uninstall_complete(
        &mut self,
        version: String,
        success: bool,
        error: Option<String>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.exclusive_op = None;

            let toast_id = state.next_toast_id();
            if success {
                let toast = Toast::success(toast_id, format!("Node {} uninstalled", version))
                    .with_undo(UndoAction::Reinstall {
                        version: version.clone(),
                    });
                state.add_toast(toast);
            } else {
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to uninstall Node {}: {}",
                        version,
                        error.unwrap_or_default()
                    ),
                ));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    fn handle_set_default(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.operation_queue.is_busy_for_exclusive() {
                let id = state.operation_queue.next_id();
                state.operation_queue.pending.push_back(QueuedOperation {
                    id,
                    request: OperationRequest::SetDefault {
                        version: version.clone(),
                    },
                    queued_at: Instant::now(),
                });
                return Task::none();
            }

            return self.start_set_default_internal(version);
        }
        Task::none()
    }

    fn start_set_default_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let previous = state
                .active_environment()
                .default_version
                .as_ref()
                .map(|v| v.to_string());

            state.operation_queue.exclusive_op = Some(Operation::SetDefault {
                version: version.clone(),
            });

            let backend = state.backend.clone();
            let version_clone = version.clone();

            return Task::perform(
                async move {
                    match backend.set_default(&version_clone).await {
                        Ok(()) => (version_clone, previous, true, None),
                        Err(e) => (version_clone, previous, false, Some(e.to_string())),
                    }
                },
                |(version, previous, success, error)| Message::DefaultChanged {
                    version,
                    previous,
                    success,
                    error,
                },
            );
        }
        Task::none()
    }

    fn handle_default_changed(
        &mut self,
        version: String,
        previous: Option<String>,
        success: bool,
        error: Option<String>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.exclusive_op = None;

            let toast_id = state.next_toast_id();
            if success {
                let mut toast =
                    Toast::success(toast_id, format!("Default set to Node {}", version));
                if let Some(prev) = previous {
                    toast = toast.with_undo(UndoAction::ResetDefault { version: prev });
                }
                state.add_toast(toast);
            } else {
                state.add_toast(Toast::error(
                    toast_id,
                    format!("Failed to set default: {}", error.unwrap_or_default()),
                ));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    fn handle_toast_undo(&mut self, id: usize) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let toast = state.toasts.iter().find(|t| t.id == id).cloned();
            state.remove_toast(id);

            if let Some(toast) = toast
                && let Some(undo_action) = toast.undo_action
            {
                match undo_action {
                    UndoAction::Reinstall { version } => {
                        return self.handle_start_install(version);
                    }
                    UndoAction::ResetDefault { version } => {
                        return self.handle_set_default(version);
                    }
                }
            }
        }
        Task::none()
    }

    fn handle_request_bulk_update_majors(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment();
            let remote = &state.available_versions.versions;

            let latest_remote_by_major: std::collections::HashMap<u32, versi_core::NodeVersion> = {
                let mut latest = std::collections::HashMap::new();
                for v in remote {
                    let major = v.version.major;
                    latest
                        .entry(major)
                        .and_modify(|existing: &mut versi_core::NodeVersion| {
                            if v.version > *existing {
                                *existing = v.version.clone();
                            }
                        })
                        .or_insert_with(|| v.version.clone());
                }
                latest
            };

            let latest_installed_by_major: std::collections::HashMap<u32, versi_core::NodeVersion> = {
                let mut latest = std::collections::HashMap::new();
                for v in &env.installed_versions {
                    let major = v.version.major;
                    latest
                        .entry(major)
                        .and_modify(|existing: &mut versi_core::NodeVersion| {
                            if v.version > *existing {
                                *existing = v.version.clone();
                            }
                        })
                        .or_insert_with(|| v.version.clone());
                }
                latest
            };

            let versions_to_update: Vec<(String, String)> = latest_installed_by_major
                .iter()
                .filter_map(|(major, installed)| {
                    latest_remote_by_major.get(major).and_then(|latest| {
                        if latest > installed {
                            Some((installed.to_string(), latest.to_string()))
                        } else {
                            None
                        }
                    })
                })
                .collect();

            if versions_to_update.is_empty() {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::success(
                    toast_id,
                    "All versions are up to date".to_string(),
                ));
                return Task::none();
            }

            state.modal = Some(Modal::ConfirmBulkUpdateMajors {
                versions: versions_to_update,
            });
        }
        Task::none()
    }

    fn handle_request_bulk_uninstall_eol(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment();
            let schedule = state.available_versions.schedule.as_ref();

            let eol_versions: Vec<String> = env
                .installed_versions
                .iter()
                .filter(|v| {
                    schedule
                        .map(|s| !s.is_active(v.version.major))
                        .unwrap_or(false)
                })
                .map(|v| v.version.to_string())
                .collect();

            if eol_versions.is_empty() {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::success(
                    toast_id,
                    "No EOL versions installed".to_string(),
                ));
                return Task::none();
            }

            state.modal = Some(Modal::ConfirmBulkUninstallEOL {
                versions: eol_versions,
            });
        }
        Task::none()
    }

    fn handle_request_bulk_uninstall_major(&mut self, major: u32) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment();

            let versions: Vec<String> = env
                .installed_versions
                .iter()
                .filter(|v| v.version.major == major)
                .map(|v| v.version.to_string())
                .collect();

            if versions.is_empty() {
                return Task::none();
            }

            state.modal = Some(Modal::ConfirmBulkUninstallMajor { major, versions });
        }
        Task::none()
    }

    fn handle_confirm_bulk_update_majors(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUpdateMajors { versions }) = state.modal.take()
        {
            for (_from, to) in versions {
                let id = state.operation_queue.next_id();
                state.operation_queue.pending.push_back(QueuedOperation {
                    id,
                    request: OperationRequest::Install {
                        version: to.clone(),
                    },
                    queued_at: Instant::now(),
                });
            }
            return self.process_next_operation();
        }
        Task::none()
    }

    fn handle_confirm_bulk_uninstall_eol(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUninstallEOL { versions }) = state.modal.take()
        {
            for version in versions {
                let id = state.operation_queue.next_id();
                state.operation_queue.pending.push_back(QueuedOperation {
                    id,
                    request: OperationRequest::Uninstall { version },
                    queued_at: Instant::now(),
                });
            }
            return self.process_next_operation();
        }
        Task::none()
    }

    fn handle_confirm_bulk_uninstall_major(&mut self, major: u32) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUninstallMajor { major: m, versions }) =
                state.modal.take()
            && m == major
        {
            for version in versions {
                let id = state.operation_queue.next_id();
                state.operation_queue.pending.push_back(QueuedOperation {
                    id,
                    request: OperationRequest::Uninstall { version },
                    queued_at: Instant::now(),
                });
            }
            return self.process_next_operation();
        }
        Task::none()
    }

    fn handle_request_bulk_uninstall_major_except_latest(&mut self, major: u32) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment();

            let mut versions_in_major: Vec<&versi_core::InstalledVersion> = env
                .installed_versions
                .iter()
                .filter(|v| v.version.major == major)
                .collect();

            versions_in_major.sort_by(|a, b| b.version.cmp(&a.version));

            if versions_in_major.len() <= 1 {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::success(
                    toast_id,
                    format!("Only one Node {}.x version installed", major),
                ));
                return Task::none();
            }

            let Some(latest) = versions_in_major.first() else {
                return Task::none();
            };
            let keeping = latest.version.to_string();

            let versions: Vec<String> = versions_in_major
                .iter()
                .skip(1)
                .map(|v| v.version.to_string())
                .collect();

            state.modal = Some(Modal::ConfirmBulkUninstallMajorExceptLatest {
                major,
                versions,
                keeping,
            });
        }
        Task::none()
    }

    fn handle_confirm_bulk_uninstall_major_except_latest(&mut self, major: u32) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(Modal::ConfirmBulkUninstallMajorExceptLatest {
                major: m, versions, ..
            }) = state.modal.take()
            && m == major
        {
            for version in versions {
                let id = state.operation_queue.next_id();
                state.operation_queue.pending.push_back(QueuedOperation {
                    id,
                    request: OperationRequest::Uninstall { version },
                    queued_at: Instant::now(),
                });
            }
            return self.process_next_operation();
        }
        Task::none()
    }

    fn process_next_operation(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.operation_queue.exclusive_op.is_some() {
                return Task::none();
            }

            let mut install_versions: Vec<String> = Vec::new();
            let mut exclusive_request: Option<OperationRequest> = None;

            while let Some(next) = state.operation_queue.pending.front() {
                match &next.request {
                    OperationRequest::Install { version } => {
                        let already_active = state.operation_queue.active_installs.iter().any(
                            |op| matches!(op, Operation::Install { version: v, .. } if v == version),
                        );
                        if !already_active && !install_versions.contains(version) {
                            install_versions.push(version.clone());
                        }
                        state.operation_queue.pending.pop_front();
                    }
                    _ => {
                        if state.operation_queue.active_installs.is_empty()
                            && install_versions.is_empty()
                            && let Some(queued) = state.operation_queue.pending.pop_front()
                        {
                            exclusive_request = Some(queued.request);
                        }
                        break;
                    }
                }
            }

            let mut tasks: Vec<Task<Message>> = Vec::new();
            for version in install_versions {
                tasks.push(self.start_install_internal(version));
            }
            if let Some(request) = exclusive_request {
                match request {
                    OperationRequest::Uninstall { version } => {
                        tasks.push(self.start_uninstall_internal(version));
                    }
                    OperationRequest::SetDefault { version } => {
                        tasks.push(self.start_set_default_internal(version));
                    }
                    OperationRequest::Install { .. } => unreachable!(),
                }
            }

            if !tasks.is_empty() {
                return Task::batch(tasks);
            }
        }
        Task::none()
    }

    fn handle_cancel_queued_operation(&mut self, id: usize) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && state.operation_queue.cancel_pending(id)
        {
            let toast_id = state.next_toast_id();
            state.add_toast(Toast::success(toast_id, "Operation cancelled".to_string()));
        }
        Task::none()
    }

    fn handle_onboarding_next(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.step = match state.step {
                OnboardingStep::Welcome => OnboardingStep::InstallFnm,
                OnboardingStep::InstallFnm => OnboardingStep::ConfigureShell,
                OnboardingStep::ConfigureShell => return self.handle_onboarding_complete(),
            };
        }
        Task::none()
    }

    fn handle_onboarding_back(&mut self) {
        if let AppState::Onboarding(state) = &mut self.state {
            state.step = match state.step {
                OnboardingStep::Welcome => OnboardingStep::Welcome,
                OnboardingStep::InstallFnm => OnboardingStep::Welcome,
                OnboardingStep::ConfigureShell => OnboardingStep::InstallFnm,
            };
        }
    }

    fn handle_onboarding_install_fnm(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.fnm_installing = true;
            state.install_error = None;

            return Task::perform(
                async move { versi_core::install_fnm().await.map_err(|e| e.to_string()) },
                Message::OnboardingFnmInstallResult,
            );
        }
        Task::none()
    }

    fn handle_onboarding_fnm_install_result(
        &mut self,
        result: Result<(), String>,
    ) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.fnm_installing = false;
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

    fn handle_onboarding_configure_shell(
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

            let shell_options = versi_shell::FnmShellOptions {
                use_on_cd: self.settings.shell_options.use_on_cd,
                resolve_engines: self.settings.shell_options.resolve_engines,
                corepack_enabled: self.settings.shell_options.corepack_enabled,
            };

            return Task::perform(
                async move {
                    use versi_shell::{ShellConfig, get_or_create_config_path};

                    let config_path = get_or_create_config_path(&shell_type)
                        .ok_or_else(|| "No config file path found".to_string())?;

                    let mut config =
                        ShellConfig::load(shell_type, config_path).map_err(|e| e.to_string())?;

                    let edit = config.add_fnm_init(&shell_options);
                    if edit.has_changes() {
                        config.apply_edit(&edit).map_err(|e| e.to_string())?;
                    }

                    Ok(())
                },
                Message::OnboardingShellConfigResult,
            );
        }
        Task::none()
    }

    fn handle_onboarding_shell_config_result(&mut self, result: Result<(), String>) {
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

    fn handle_onboarding_complete(&mut self) -> Task<Message> {
        let fnm_path = PathBuf::from("fnm");
        let fnm_dir = versi_core::detect_fnm_dir();

        let backend = FnmBackend::new(fnm_path.clone(), None, fnm_dir.clone());
        let backend = if let Some(dir) = fnm_dir.clone() {
            backend.with_fnm_dir(dir)
        } else {
            backend
        };
        let backend: Box<dyn VersionManager> = Box::new(backend.clone());
        let mut main_state = MainState::new(backend, None);
        main_state.search_query = "lts".to_string();
        self.state = AppState::Main(main_state);

        let load_backend = FnmBackend::new(fnm_path, None, fnm_dir.clone());
        let load_backend = if let Some(dir) = fnm_dir {
            load_backend.with_fnm_dir(dir)
        } else {
            load_backend
        };
        let load_task = Task::perform(
            async move {
                let versions = load_backend.list_installed().await.unwrap_or_default();
                let default = load_backend.default_version().await.ok().flatten();
                (versions, default)
            },
            move |(versions, default)| Message::EnvironmentLoaded {
                env_id: EnvironmentId::Native,
                versions,
                default_version: default,
            },
        );

        let fetch_remote = self.handle_fetch_remote_versions();
        let fetch_schedule = self.handle_fetch_release_schedule();

        Task::batch([load_task, fetch_remote, fetch_schedule])
    }

    fn handle_check_shell_setup(&mut self) -> Task<Message> {
        use versi_shell::{detect_native_shells, verify_shell_config};
        #[cfg(target_os = "windows")]
        use versi_shell::{detect_wsl_shells, verify_wsl_shell_config};

        #[allow(unused_variables)]
        let env_id = if let AppState::Main(state) = &self.state {
            Some(state.active_environment().id.clone())
        } else {
            None
        };

        Task::perform(
            async move {
                #[cfg(target_os = "windows")]
                let (shells, wsl_distro) = match &env_id {
                    Some(EnvironmentId::Wsl { distro, .. }) => {
                        (detect_wsl_shells(distro), Some(distro.clone()))
                    }
                    _ => (detect_native_shells(), None),
                };
                #[cfg(not(target_os = "windows"))]
                let (shells, wsl_distro): (Vec<_>, Option<String>) = (detect_native_shells(), None);

                let mut results = Vec::new();

                for shell in shells {
                    #[cfg(target_os = "windows")]
                    let result = if let Some(ref distro) = wsl_distro {
                        verify_wsl_shell_config(&shell.shell_type, distro).await
                    } else {
                        verify_shell_config(&shell.shell_type).await
                    };
                    #[cfg(not(target_os = "windows"))]
                    let result = {
                        let _ = &wsl_distro;
                        verify_shell_config(&shell.shell_type).await
                    };
                    results.push((shell.shell_type, result));
                }

                results
            },
            Message::ShellSetupChecked,
        )
    }

    fn handle_shell_setup_checked(
        &mut self,
        results: Vec<(versi_shell::ShellType, versi_shell::VerificationResult)>,
    ) {
        let mut first_detected_options: Option<versi_shell::FnmShellOptions> = None;

        if let AppState::Main(state) = &mut self.state {
            state.settings_state.checking_shells = false;
            state.settings_state.shell_statuses = results
                .into_iter()
                .map(|(shell_type, result)| {
                    let status = match result {
                        versi_shell::VerificationResult::Configured(options) => {
                            if first_detected_options.is_none() {
                                first_detected_options = options;
                            }
                            ShellVerificationStatus::Configured
                        }
                        versi_shell::VerificationResult::NotConfigured => {
                            ShellVerificationStatus::NotConfigured
                        }
                        versi_shell::VerificationResult::ConfigFileNotFound => {
                            ShellVerificationStatus::NoConfigFile
                        }
                        versi_shell::VerificationResult::FunctionalButNotInConfig => {
                            ShellVerificationStatus::FunctionalButNotInConfig
                        }
                        versi_shell::VerificationResult::Error(e) => {
                            ShellVerificationStatus::Error(e)
                        }
                    };
                    ShellSetupStatus {
                        shell_name: shell_type.name().to_string(),
                        shell_type,
                        status,
                        configuring: false,
                    }
                })
                .collect();
        }

        if let Some(options) = first_detected_options {
            self.settings.shell_options.use_on_cd = options.use_on_cd;
            self.settings.shell_options.resolve_engines = options.resolve_engines;
            self.settings.shell_options.corepack_enabled = options.corepack_enabled;
        }
    }

    fn handle_configure_shell(&mut self, shell_type: versi_shell::ShellType) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(shell) = state
                .settings_state
                .shell_statuses
                .iter_mut()
                .find(|s| s.shell_type == shell_type)
        {
            shell.configuring = true;
        }

        let shell_options = versi_shell::FnmShellOptions {
            use_on_cd: self.settings.shell_options.use_on_cd,
            resolve_engines: self.settings.shell_options.resolve_engines,
            corepack_enabled: self.settings.shell_options.corepack_enabled,
        };

        let shell_type_for_callback = shell_type.clone();
        Task::perform(
            async move {
                use versi_shell::{ShellConfig, get_or_create_config_path};

                let config_path = get_or_create_config_path(&shell_type)
                    .ok_or_else(|| "No config file path found".to_string())?;

                let mut config = ShellConfig::load(shell_type.clone(), config_path)
                    .map_err(|e| e.to_string())?;

                let edit = config.add_fnm_init(&shell_options);
                if edit.has_changes() {
                    config.apply_edit(&edit).map_err(|e| e.to_string())?;
                }

                Ok::<_, String>(())
            },
            move |result| Message::ShellConfigured(shell_type_for_callback.clone(), result),
        )
    }

    fn handle_shell_configured(
        &mut self,
        shell_type: versi_shell::ShellType,
        result: Result<(), String>,
    ) {
        if let AppState::Main(state) = &mut self.state
            && let Some(shell) = state
                .settings_state
                .shell_statuses
                .iter_mut()
                .find(|s| s.shell_type == shell_type)
        {
            shell.configuring = false;
            match result {
                Ok(()) => shell.status = ShellVerificationStatus::Configured,
                Err(e) => shell.status = ShellVerificationStatus::Error(e),
            }
        }
    }

    fn update_shell_flags(&self) -> Task<Message> {
        let shell_options = versi_shell::FnmShellOptions {
            use_on_cd: self.settings.shell_options.use_on_cd,
            resolve_engines: self.settings.shell_options.resolve_engines,
            corepack_enabled: self.settings.shell_options.corepack_enabled,
        };

        Task::perform(
            async move {
                use versi_shell::ShellConfig;

                let shells = detect_shells();
                let mut updated_count = 0;

                for shell in shells {
                    if let Some(config_path) = shell.config_file
                        && let Ok(mut config) =
                            ShellConfig::load(shell.shell_type.clone(), config_path)
                        && config.has_fnm_init()
                    {
                        let edit = config.update_fnm_flags(&shell_options);
                        if edit.has_changes() {
                            config.apply_edit(&edit).map_err(|e| e.to_string())?;
                            updated_count += 1;
                        }
                    }
                }

                Ok::<_, String>(updated_count)
            },
            Message::ShellFlagsUpdated,
        )
    }

    fn handle_check_for_app_update(&mut self) -> Task<Message> {
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let client = self.http_client.clone();
        Task::perform(
            async move { check_for_update(&client, &current_version).await },
            Message::AppUpdateChecked,
        )
    }

    fn handle_app_update_checked(&mut self, result: Result<Option<versi_core::AppUpdate>, String>) {
        if let AppState::Main(state) = &mut self.state {
            match result {
                Ok(update) => state.app_update = update,
                Err(e) => debug!("App update check failed: {}", e),
            }
        }
    }

    fn handle_check_for_fnm_update(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &self.state
            && let Some(version) = &state.active_environment().fnm_version
        {
            let version = version.clone();
            let client = self.http_client.clone();
            return Task::perform(
                async move { check_for_fnm_update(&client, &version).await },
                Message::FnmUpdateChecked,
            );
        }
        Task::none()
    }

    fn handle_fnm_update_checked(&mut self, result: Result<Option<versi_core::FnmUpdate>, String>) {
        if let AppState::Main(state) = &mut self.state {
            match result {
                Ok(update) => state.fnm_update = update,
                Err(e) => debug!("fnm update check failed: {}", e),
            }
        }
    }

    fn handle_tray_event(&mut self, msg: TrayMessage) -> Task<Message> {
        match msg {
            TrayMessage::ShowWindow => {
                if let Some(id) = self.window_id {
                    set_dock_visible(true);

                    let needs_refresh = if let AppState::Main(state) = &self.state {
                        state.active_environment().installed_versions.is_empty()
                            && !state.active_environment().loading
                    } else {
                        false
                    };

                    let mut tasks = vec![
                        iced::window::set_mode(id, iced::window::Mode::Windowed),
                        iced::window::minimize(id, false),
                        iced::window::gain_focus(id),
                    ];

                    if needs_refresh {
                        tasks.push(Task::done(Message::RefreshEnvironment));
                    }

                    Task::batch(tasks)
                } else {
                    Task::none()
                }
            }
            TrayMessage::OpenSettings => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Settings;
                    state.settings_state.checking_shells = true;
                }
                let show_task = if let Some(id) = self.window_id {
                    set_dock_visible(true);
                    Task::batch([
                        iced::window::set_mode(id, iced::window::Mode::Windowed),
                        iced::window::minimize(id, false),
                        iced::window::gain_focus(id),
                    ])
                } else {
                    Task::none()
                };
                let shell_task = self.handle_check_shell_setup();
                let log_stats_task = Task::perform(
                    async {
                        let log_path = versi_platform::AppPaths::new().log_file();
                        std::fs::metadata(&log_path).ok().map(|m| m.len())
                    },
                    Message::LogFileStatsLoaded,
                );
                Task::batch([show_task, shell_task, log_stats_task])
            }
            TrayMessage::OpenAbout => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::About;
                }
                if let Some(id) = self.window_id {
                    set_dock_visible(true);
                    Task::batch([
                        iced::window::set_mode(id, iced::window::Mode::Windowed),
                        iced::window::minimize(id, false),
                        iced::window::gain_focus(id),
                    ])
                } else {
                    Task::none()
                }
            }
            TrayMessage::Quit => iced::exit(),
            TrayMessage::SetDefault { env_index, version } => {
                if let AppState::Main(state) = &mut self.state
                    && env_index != state.active_environment_idx
                {
                    state.active_environment_idx = env_index;
                    let env = &state.environments[env_index];
                    let env_id = env.id.clone();
                    state.backend =
                        create_backend_for_environment(&env_id, &self.fnm_path, &self.fnm_dir);
                }
                self.handle_set_default(version)
            }
        }
    }

    fn handle_tray_behavior_changed(&mut self, behavior: TrayBehavior) -> Task<Message> {
        let old_behavior = self.settings.tray_behavior.clone();
        self.settings.tray_behavior = behavior.clone();
        let _ = self.settings.save();

        if old_behavior == TrayBehavior::Disabled && behavior != TrayBehavior::Disabled {
            if let Err(e) = tray::init_tray(&behavior) {
                error!("Failed to initialize tray: {}", e);
            } else {
                self.update_tray_menu();
            }
        } else if behavior == TrayBehavior::Disabled {
            tray::destroy_tray();
        }

        Task::none()
    }

    fn update_tray_menu(&self) {
        if let AppState::Main(state) = &self.state {
            let data = TrayMenuData::from_environments(&state.environments);
            tray::update_menu(&data);
        }
    }
}

async fn initialize() -> InitResult {
    use crate::message::EnvironmentInfo;

    info!("Initializing application...");

    debug!("Detecting fnm installation...");
    let detection = detect_fnm().await;
    info!(
        "fnm detection result: found={}, path={:?}, version={:?}",
        detection.found, detection.path, detection.version
    );

    #[allow(unused_mut)]
    let mut environments = vec![EnvironmentInfo {
        id: EnvironmentId::Native,
        fnm_version: detection.version.clone(),
        available: true,
        unavailable_reason: None,
    }];

    #[cfg(windows)]
    {
        use versi_platform::detect_wsl_distros;
        info!("Running on Windows, detecting WSL distros...");
        let distros = detect_wsl_distros();
        debug!(
            "WSL distros found: {:?}",
            distros.iter().map(|d| &d.name).collect::<Vec<_>>()
        );

        for distro in distros {
            if !distro.is_running {
                info!(
                    "Adding unavailable WSL environment: {} (not running)",
                    distro.name
                );
                environments.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        fnm_path: String::new(),
                    },
                    fnm_version: None,
                    available: false,
                    unavailable_reason: Some("Not running".to_string()),
                });
            } else if let Some(fnm_path) = distro.fnm_path {
                info!(
                    "Adding WSL environment: {} (fnm at {})",
                    distro.name, fnm_path
                );
                let fnm_version = get_wsl_fnm_version(&distro.name, &fnm_path).await;
                environments.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        fnm_path,
                    },
                    fnm_version,
                    available: true,
                    unavailable_reason: None,
                });
            } else {
                info!(
                    "Adding unavailable WSL environment: {} (fnm not found)",
                    distro.name
                );
                environments.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        fnm_path: String::new(),
                    },
                    fnm_version: None,
                    available: false,
                    unavailable_reason: Some("fnm not installed".to_string()),
                });
            }
        }
    }

    info!(
        "Initialization complete with {} environments",
        environments.len()
    );
    for (i, env) in environments.iter().enumerate() {
        trace!("  Environment {}: {:?}", i, env);
    }

    InitResult {
        fnm_found: detection.found,
        fnm_path: detection.path,
        fnm_dir: detection.fnm_dir,
        fnm_version: detection.version,
        environments,
    }
}

#[cfg(windows)]
async fn get_wsl_fnm_version(distro: &str, fnm_path: &str) -> Option<String> {
    use tokio::process::Command;

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", fnm_path, "--version"])
        .hide_window()
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = version_str
            .trim()
            .strip_prefix("fnm ")
            .unwrap_or(version_str.trim())
            .to_string();
        debug!("WSL {} fnm version: {}", distro, version);
        Some(version)
    } else {
        None
    }
}

fn create_backend_for_environment(
    env_id: &EnvironmentId,
    detected_fnm_path: &Path,
    detected_fnm_dir: &Option<PathBuf>,
) -> Box<dyn VersionManager> {
    match env_id {
        EnvironmentId::Native => {
            let backend = FnmBackend::new(
                detected_fnm_path.to_path_buf(),
                None,
                detected_fnm_dir.clone(),
            );
            let backend = if let Some(dir) = detected_fnm_dir {
                backend.with_fnm_dir(dir.clone())
            } else {
                backend
            };
            Box::new(backend)
        }
        EnvironmentId::Wsl { distro, fnm_path } => {
            Box::new(FnmBackend::with_wsl(distro.clone(), fnm_path.clone()))
        }
    }
}

fn reveal_in_file_manager(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .args(["/select,", &path.to_string_lossy()])
            .hide_window()
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
}
