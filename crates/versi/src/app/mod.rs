mod environment;
mod init;
mod onboarding;
mod operations;
mod platform;
mod shell;
mod tray_handlers;
mod versions;

use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use iced::{Element, Subscription, Task, Theme};

use versi_backend::BackendProvider;

use crate::message::Message;
use crate::settings::{AppSettings, ThemeSetting, TrayBehavior};
use crate::state::{AppState, MainViewKind};
use crate::theme::{dark_theme, get_system_theme, light_theme};
use crate::tray;
use crate::views;

pub struct Versi {
    pub(crate) state: AppState,
    pub(crate) settings: AppSettings,
    pub(crate) window_id: Option<iced::window::Id>,
    pub(crate) pending_minimize: bool,
    pub(crate) backend_path: PathBuf,
    pub(crate) backend_dir: Option<PathBuf>,
    pub(crate) window_size: Option<iced::Size>,
    pub(crate) window_position: Option<iced::Point>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) providers: HashMap<&'static str, Arc<dyn BackendProvider>>,
    pub(crate) provider: Arc<dyn BackendProvider>,
}

impl Versi {
    pub fn new() -> (Self, Task<Message>) {
        let settings = AppSettings::load();

        let should_minimize =
            settings.start_minimized && settings.tray_behavior != TrayBehavior::Disabled;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(format!("versi/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();

        let fnm_provider: Arc<dyn BackendProvider> = Arc::new(versi_core::FnmProvider::new());
        let nvm_provider: Arc<dyn BackendProvider> = Arc::new(versi_nvm::NvmProvider::new());

        let mut providers: HashMap<&'static str, Arc<dyn BackendProvider>> = HashMap::new();
        providers.insert(fnm_provider.name(), fnm_provider.clone());
        providers.insert(nvm_provider.name(), nvm_provider.clone());

        let preferred = settings.preferred_backend.as_deref().unwrap_or("fnm");
        let active_provider = providers.get(preferred).cloned().unwrap_or(fnm_provider);

        let app = Self {
            state: AppState::Loading,
            settings,
            window_id: None,
            pending_minimize: should_minimize,
            backend_path: PathBuf::from(active_provider.name()),
            backend_dir: None,
            window_size: None,
            window_position: None,
            http_client,
            providers: providers.clone(),
            provider: active_provider,
        };

        let all_providers: Vec<Arc<dyn BackendProvider>> = providers.values().cloned().collect();
        let preferred_backend = app.settings.preferred_backend.clone();
        let init_task = Task::perform(
            init::initialize(all_providers, preferred_backend),
            Message::Initialized,
        );

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
            Message::EnvironmentLoaded { env_id, versions } => {
                self.handle_environment_loaded(env_id, versions)
            }
            Message::RefreshEnvironment => self.handle_refresh_environment(),
            Message::FocusSearch => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Versions;
                }
                iced::widget::operation::focus(iced::widget::Id::new(
                    crate::views::main_view::search::SEARCH_INPUT_ID,
                ))
            }
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
            Message::RequestUninstall(version) => self.handle_uninstall(version),
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
            Message::DefaultChanged { success, error } => {
                self.handle_default_changed(success, error)
            }
            Message::ToastDismiss(id) => {
                if let AppState::Main(state) = &mut self.state {
                    state.remove_toast(id);
                }
                Task::none()
            }
            Message::NavigateToVersions => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Versions;
                }
                Task::none()
            }
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
                Task::perform(
                    async move { platform::reveal_in_file_manager(&log_path) },
                    |_| Message::NoOp,
                )
            }
            Message::LogFileStatsLoaded(size) => {
                if let AppState::Main(state) = &mut self.state {
                    state.settings_state.log_file_size = size;
                }
                Task::none()
            }
            Message::ShellFlagsUpdated => Task::none(),
            Message::ShellSetupChecked(results) => {
                self.handle_shell_setup_checked(results);
                Task::none()
            }
            Message::ConfigureShell(shell_type) => self.handle_configure_shell(shell_type),
            Message::ShellConfigured(shell_type, result) => {
                self.handle_shell_configured(shell_type, result);
                Task::none()
            }
            Message::PreferredBackendChanged(name) => self.handle_preferred_backend_changed(name),
            Message::OnboardingNext => self.handle_onboarding_next(),
            Message::OnboardingBack => {
                self.handle_onboarding_back();
                Task::none()
            }
            Message::OnboardingSelectBackend(name) => {
                self.handle_onboarding_select_backend(name);
                Task::none()
            }
            Message::OnboardingInstallBackend => self.handle_onboarding_install_backend(),
            Message::OnboardingBackendInstallResult(result) => {
                self.handle_onboarding_backend_install_result(result)
            }
            Message::OnboardingConfigureShell(shell_type) => {
                self.handle_onboarding_configure_shell(shell_type)
            }
            Message::OnboardingShellConfigResult(result) => {
                self.handle_onboarding_shell_config_result(result);
                Task::none()
            }
            Message::OnboardingComplete => self.handle_onboarding_complete(),
            Message::AnimationTick => {
                if let AppState::Main(state) = &mut self.state {
                    let loading = state.active_environment().loading;
                    state.refresh_rotation += std::f32::consts::TAU / 40.0;
                    if !loading && state.refresh_rotation >= std::f32::consts::TAU {
                        state.refresh_rotation = 0.0;
                    }
                }
                Task::none()
            }
            Message::Tick => {
                if let AppState::Main(state) = &mut self.state {
                    state.toasts.retain(|t| !t.is_expired());
                }
                Task::none()
            }
            Message::WindowEvent(iced::window::Event::CloseRequested)
            | Message::WindowEvent(iced::window::Event::Closed)
            | Message::CloseWindow => {
                self.save_window_geometry();
                if self.settings.tray_behavior == TrayBehavior::AlwaysRunning
                    && tray::is_tray_active()
                {
                    if let Some(id) = self.window_id {
                        platform::set_dock_visible(false);
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
                platform::set_dock_visible(false);
                Task::none()
            }
            Message::WindowEvent(_) => Task::none(),
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
            Message::BackendUpdateChecked(result) => {
                self.handle_backend_update_checked(result);
                Task::none()
            }
            Message::FetchReleaseSchedule => self.handle_fetch_release_schedule(),
            Message::OpenBackendUpdate => {
                if let AppState::Main(state) = &self.state
                    && let Some(update) = &state.backend_update
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
            AppState::Onboarding(state) => {
                let backend_name = state
                    .selected_backend
                    .as_deref()
                    .unwrap_or(self.provider.name());
                views::onboarding::view(state, backend_name)
            }
            AppState::Main(state) => match state.view {
                MainViewKind::Versions => views::main_view::view(state, &self.settings),
                MainViewKind::Settings => {
                    views::settings_view::view(&state.settings_state, &self.settings, state)
                }
                MainViewKind::About => views::about_view::view(state),
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
                let cmd = modifiers.command();
                #[cfg(not(target_os = "macos"))]
                let cmd = modifiers.control();

                if cmd && let iced::keyboard::Key::Character(c) = &key {
                    match c.as_str() {
                        "k" => return Some(Message::FocusSearch),
                        "," => return Some(Message::NavigateToSettings),
                        "r" => return Some(Message::RefreshEnvironment),
                        "w" => return Some(Message::CloseWindow),
                        _ => {}
                    }
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

        let animation_tick = if self.is_refresh_animating() {
            iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::AnimationTick)
        } else {
            Subscription::none()
        };

        Subscription::batch([
            tick,
            keyboard,
            window_events,
            tray_sub,
            window_open_sub,
            animation_tick,
        ])
    }

    fn is_refresh_animating(&self) -> bool {
        if let AppState::Main(state) = &self.state {
            state.refresh_rotation != 0.0
        } else {
            false
        }
    }

    fn handle_preferred_backend_changed(&mut self, name: String) -> Task<Message> {
        self.settings.preferred_backend = Some(name.clone());
        let _ = self.settings.save();

        if let AppState::Main(state) = &mut self.state {
            let is_detected = state.detected_backends.contains(&name.as_str());
            if is_detected && state.backend_name != name.as_str() {
                if let Some(provider) = self.providers.get(name.as_str()) {
                    self.provider = provider.clone();
                }
                let all_providers = self.all_providers();
                let preferred = self.settings.preferred_backend.clone();
                self.state = AppState::Loading;
                return Task::perform(
                    init::initialize(all_providers, preferred),
                    Message::Initialized,
                );
            }
        }

        Task::none()
    }

    pub(crate) fn all_providers(&self) -> Vec<Arc<dyn BackendProvider>> {
        self.providers.values().cloned().collect()
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
}
