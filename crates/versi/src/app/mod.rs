mod auto_update;
mod bulk_operations;
mod environment;
mod init;
mod onboarding;
mod operations;
mod platform;
mod shell;
mod tray_handlers;
mod versions;
mod window;

use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use iced::{Element, Subscription, Task, Theme};

use versi_backend::BackendProvider;

use crate::message::Message;
use crate::settings::{AppSettings, ThemeSetting, TrayBehavior};
use crate::state::{AppState, MainViewKind};
use crate::theme::{dark_theme, light_theme};
use crate::tray;
use crate::views;

pub struct Versi {
    pub(crate) state: AppState,
    pub(crate) settings: AppSettings,
    pub(crate) window_id: Option<iced::window::Id>,
    pub(crate) pending_minimize: bool,
    pub(crate) pending_show: bool,
    pub(crate) window_visible: bool,
    pub(crate) backend_path: PathBuf,
    pub(crate) backend_dir: Option<PathBuf>,
    pub(crate) window_size: Option<iced::Size>,
    pub(crate) window_position: Option<iced::Point>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) providers: HashMap<&'static str, Arc<dyn BackendProvider>>,
    pub(crate) provider: Arc<dyn BackendProvider>,
    pub(crate) system_theme_mode: iced::theme::Mode,
}

impl Versi {
    pub fn new() -> (Self, Task<Message>) {
        let settings = AppSettings::load();

        let should_minimize = settings.start_minimized
            && settings.tray_behavior != TrayBehavior::Disabled
            && tray::is_tray_active();

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(settings.http_timeout_secs))
            .user_agent(format!("versi/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();

        let fnm_provider: Arc<dyn BackendProvider> = Arc::new(versi_fnm::FnmProvider::new());
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
            pending_show: false,
            window_visible: !should_minimize,
            backend_path: PathBuf::from(active_provider.name()),
            backend_dir: None,
            window_size: None,
            window_position: None,
            http_client,
            providers: providers.clone(),
            provider: active_provider,
            system_theme_mode: iced::theme::Mode::None,
        };

        let all_providers: Vec<Arc<dyn BackendProvider>> = providers.values().cloned().collect();
        let preferred_backend = app.settings.preferred_backend.clone();
        let init_task = Task::perform(
            init::initialize(all_providers, preferred_backend),
            Message::Initialized,
        );
        let theme_task = iced::system::theme().map(Message::SystemThemeChanged);

        (app, Task::batch([init_task, theme_task]))
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
            Message::SelectPreviousVersion => {
                if let AppState::Main(state) = &mut self.state
                    && state.view == MainViewKind::Versions
                    && state.modal.is_none()
                {
                    let versions = state.navigable_versions(self.settings.search_results_limit);
                    if !versions.is_empty() {
                        let new_idx = match &state.hovered_version {
                            Some(current) => versions
                                .iter()
                                .position(|v| v == current)
                                .map(|i| i.saturating_sub(1))
                                .unwrap_or(0),
                            None => versions.len() - 1,
                        };
                        state.hovered_version = Some(versions[new_idx].clone());
                    }
                }
                Task::none()
            }
            Message::SelectNextVersion => {
                if let AppState::Main(state) = &mut self.state
                    && state.view == MainViewKind::Versions
                    && state.modal.is_none()
                {
                    let versions = state.navigable_versions(self.settings.search_results_limit);
                    if !versions.is_empty() {
                        let new_idx = match &state.hovered_version {
                            Some(current) => versions
                                .iter()
                                .position(|v| v == current)
                                .map(|i| (i + 1).min(versions.len() - 1))
                                .unwrap_or(0),
                            None => 0,
                        };
                        state.hovered_version = Some(versions[new_idx].clone());
                    }
                }
                Task::none()
            }
            Message::ActivateSelectedVersion => {
                if let AppState::Main(state) = &self.state
                    && state.view == MainViewKind::Versions
                    && state.modal.is_none()
                    && let Some(version) = state.hovered_version.clone()
                {
                    if state.is_version_installed(&version) {
                        return self.update(Message::SetDefault(version));
                    } else {
                        return self.update(Message::StartInstall(version));
                    }
                }
                Task::none()
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
                        let log_path = versi_platform::AppPaths::new().ok()?.log_file();
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
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                Task::none()
            }
            Message::ShellOptionUseOnCdToggled(value) => {
                self.settings
                    .shell_options_for_mut(self.provider.name())
                    .use_on_cd = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                self.update_shell_flags()
            }
            Message::ShellOptionResolveEnginesToggled(value) => {
                self.settings
                    .shell_options_for_mut(self.provider.name())
                    .resolve_engines = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                self.update_shell_flags()
            }
            Message::ShellOptionCorepackEnabledToggled(value) => {
                self.settings
                    .shell_options_for_mut(self.provider.name())
                    .corepack_enabled = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                self.update_shell_flags()
            }
            Message::DebugLoggingToggled(value) => {
                self.settings.debug_logging = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                crate::logging::set_logging_enabled(value);
                if value {
                    info!("Debug logging enabled");
                }
                Task::none()
            }
            Message::CopyToClipboard(text) => iced::clipboard::write(text),
            Message::ClearLogFile => {
                let Some(log_path) = versi_platform::AppPaths::new()
                    .ok()
                    .map(|p| p.log_file())
                else {
                    return Task::none();
                };
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
                let Some(log_path) = versi_platform::AppPaths::new()
                    .ok()
                    .map(|p| p.log_file())
                else {
                    return Task::none();
                };
                Task::perform(
                    async move { platform::reveal_in_file_manager(&log_path) },
                    |_| Message::NoOp,
                )
            }
            Message::RevealSettingsFile => {
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                let Some(settings_path) = versi_platform::AppPaths::new()
                    .ok()
                    .map(|p| p.settings_file())
                else {
                    return Task::none();
                };
                Task::perform(
                    async move { platform::reveal_in_file_manager(&settings_path) },
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
            Message::ExportSettings => {
                let settings = self.settings.clone();
                Task::perform(
                    async move {
                        let dialog = rfd::AsyncFileDialog::new()
                            .set_file_name("versi-settings.json")
                            .add_filter("JSON", &["json"])
                            .save_file()
                            .await;
                        match dialog {
                            Some(handle) => {
                                let content = serde_json::to_string_pretty(&settings)
                                    .map_err(|e| e.to_string())?;
                                let path = handle.path().to_path_buf();
                                tokio::fs::write(&path, content)
                                    .await
                                    .map_err(|e| e.to_string())?;
                                Ok(path)
                            }
                            None => Err("Cancelled".to_string()),
                        }
                    },
                    Message::SettingsExported,
                )
            }
            Message::SettingsExported(result) => {
                if let Err(e) = result
                    && e != "Cancelled"
                    && let AppState::Main(state) = &mut self.state
                {
                    let id = state.next_toast_id();
                    state.add_toast(crate::state::Toast::error(
                        id,
                        format!("Export failed: {}", e),
                    ));
                }
                Task::none()
            }
            Message::ImportSettings => Task::perform(
                async {
                    let dialog = rfd::AsyncFileDialog::new()
                        .add_filter("JSON", &["json"])
                        .pick_file()
                        .await;
                    match dialog {
                        Some(handle) => {
                            let content = tokio::fs::read_to_string(handle.path())
                                .await
                                .map_err(|e| e.to_string())?;
                            let imported: crate::settings::AppSettings =
                                serde_json::from_str(&content).map_err(|e| e.to_string())?;
                            imported.save().map_err(|e| e.to_string())?;
                            Ok(())
                        }
                        None => Err("Cancelled".to_string()),
                    }
                },
                Message::SettingsImported,
            ),
            Message::SettingsImported(result) => {
                match result {
                    Ok(()) => {
                        self.settings = crate::settings::AppSettings::load();
                    }
                    Err(e) if e != "Cancelled" => {
                        if let AppState::Main(state) = &mut self.state {
                            let id = state.next_toast_id();
                            state.add_toast(crate::state::Toast::error(
                                id,
                                format!("Import failed: {}", e),
                            ));
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
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
                #[cfg(target_os = "linux")]
                {
                    if tray::is_tray_active() {
                        while gtk::events_pending() {
                            gtk::main_iteration();
                        }
                    }
                }
                if let AppState::Main(state) = &mut self.state {
                    let timeout = self.settings.toast_timeout_secs;
                    state.toasts.retain(|t| !t.is_expired(timeout));
                }
                Task::none()
            }
            Message::WindowEvent(iced::window::Event::CloseRequested)
            | Message::WindowEvent(iced::window::Event::Closed)
            | Message::CloseWindow => self.handle_window_close(),
            Message::WindowEvent(iced::window::Event::Resized(size)) => {
                self.window_size = Some(size);
                Task::none()
            }
            Message::WindowEvent(iced::window::Event::Moved(point)) => {
                self.window_position = Some(point);
                Task::none()
            }
            Message::WindowOpened(id) => self.handle_window_opened(id),
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
            Message::StartAppUpdate => self.handle_start_app_update(),
            Message::AppUpdateProgress { downloaded, total } => {
                self.handle_app_update_progress(downloaded, total);
                Task::none()
            }
            Message::AppUpdateExtracting => {
                self.handle_app_update_extracting();
                Task::none()
            }
            Message::AppUpdateApplying => {
                self.handle_app_update_applying();
                Task::none()
            }
            Message::AppUpdateComplete(result) => self.handle_app_update_complete(result),
            Message::RestartApp => self.handle_restart_app(),
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
            Message::ShowKeyboardShortcuts => {
                if let AppState::Main(state) = &mut self.state {
                    state.modal = Some(crate::state::Modal::KeyboardShortcuts);
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
            Message::SelectNextEnvironment => {
                if let AppState::Main(state) = &self.state
                    && state.environments.len() > 1
                {
                    let next = (state.active_environment_idx + 1) % state.environments.len();
                    return self.handle_environment_selected(next);
                }
                Task::none()
            }
            Message::SelectPreviousEnvironment => {
                if let AppState::Main(state) = &self.state
                    && state.environments.len() > 1
                {
                    let prev = if state.active_environment_idx == 0 {
                        state.environments.len() - 1
                    } else {
                        state.active_environment_idx - 1
                    };
                    return self.handle_environment_selected(prev);
                }
                Task::none()
            }
            Message::TrayEvent(tray_msg) => self.handle_tray_event(tray_msg),
            Message::TrayBehaviorChanged(behavior) => self.handle_tray_behavior_changed(behavior),
            Message::StartMinimizedToggled(value) => {
                self.settings.start_minimized = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                Task::none()
            }
            Message::SystemThemeChanged(mode) => {
                self.system_theme_mode = mode;
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
            AppState::Main(state) => {
                use iced::widget::{column, container};

                let tab_row = views::main_view::tabs::environment_tabs_view(state);
                let has_tabs = tab_row.is_some();

                let inner = match state.view {
                    MainViewKind::Versions => {
                        views::main_view::view(state, &self.settings, has_tabs)
                    }
                    MainViewKind::Settings => views::settings_view::view(
                        &state.settings_state,
                        &self.settings,
                        state,
                        has_tabs,
                        self.is_system_dark(),
                    ),
                    MainViewKind::About => views::about_view::view(state, has_tabs),
                };

                if let Some(tabs) = tab_row {
                    let tabs_container = container(tabs)
                        .padding(iced::Padding::new(0.0).top(12.0).left(24.0).right(24.0));
                    column![tabs_container, inner].spacing(0).into()
                } else {
                    inner
                }
            }
        }
    }

    pub fn theme(&self) -> Theme {
        match self.settings.theme {
            ThemeSetting::System => {
                if self.system_theme_mode == iced::theme::Mode::Dark {
                    dark_theme()
                } else {
                    light_theme()
                }
            }
            ThemeSetting::Light => light_theme(),
            ThemeSetting::Dark => dark_theme(),
        }
    }

    pub fn is_system_dark(&self) -> bool {
        self.system_theme_mode == iced::theme::Mode::Dark
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick_ms = {
            #[cfg(target_os = "linux")]
            {
                if tray::is_tray_active() { 100 } else { 1000 }
            }
            #[cfg(not(target_os = "linux"))]
            {
                1000u64
            }
        };
        let tick =
            iced::time::every(std::time::Duration::from_millis(tick_ms)).map(|_| Message::Tick);

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

                if !cmd
                    && let iced::keyboard::Key::Character(c) = &key
                    && c.as_str() == "?"
                {
                    return Some(Message::ShowKeyboardShortcuts);
                }

                if let iced::keyboard::Key::Named(named) = &key {
                    match named {
                        iced::keyboard::key::Named::ArrowUp => {
                            return Some(Message::SelectPreviousVersion);
                        }
                        iced::keyboard::key::Named::ArrowDown => {
                            return Some(Message::SelectNextVersion);
                        }
                        iced::keyboard::key::Named::Enter => {
                            return Some(Message::ActivateSelectedVersion);
                        }
                        iced::keyboard::key::Named::Tab if cmd && modifiers.shift() => {
                            return Some(Message::SelectPreviousEnvironment);
                        }
                        iced::keyboard::key::Named::Tab if cmd => {
                            return Some(Message::SelectNextEnvironment);
                        }
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

        let tray_sub =
            if self.settings.tray_behavior != TrayBehavior::Disabled && tray::is_tray_active() {
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

        let theme_changes = iced::system::theme_changes().map(Message::SystemThemeChanged);

        Subscription::batch([
            tick,
            keyboard,
            window_events,
            tray_sub,
            window_open_sub,
            animation_tick,
            theme_changes,
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
        if let Err(e) = self.settings.save() {
            log::error!("Failed to save settings: {e}");
        }

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
}
