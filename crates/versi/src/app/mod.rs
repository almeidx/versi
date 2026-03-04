mod async_helpers;
mod auto_update;
mod bulk_operations;
mod environment;
mod init;
mod onboarding;
mod operations;
mod platform;
mod settings_io;
mod settings_save;
mod shell;
mod tray_handlers;
mod update;
mod versions;
mod window;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use iced::{Element, Subscription, Task, Theme};

use versi_backend::BackendProvider;

use crate::backend_kind::BackendKind;
use crate::message::Message;
use crate::settings::{AppSettings, ThemeSetting, TrayBehavior};
use crate::state::{AppState, MainViewKind};
#[cfg(test)]
use crate::state::{EnvironmentState, MainState, OnboardingState};
use crate::theme::{dark_theme, light_theme};
use crate::tray;
use crate::views;
#[cfg(test)]
use versi_backend::BackendDetection;
#[cfg(test)]
use versi_platform::EnvironmentId;

#[cfg(target_os = "linux")]
const TICK_INTERVAL_FAST_MS: u64 = 100;
const TICK_INTERVAL_DEFAULT_MS: u64 = 1000;

fn should_dismiss_context_menu(message: &Message) -> bool {
    !matches!(
        message,
        Message::NoOp
            | Message::InitTray
            | Message::Tick
            | Message::AnimationTick
            | Message::VersionListCursorMoved(_)
            | Message::VersionRowHovered(_)
            | Message::InstallProgress { .. }
            | Message::WindowEvent(_)
            | Message::SystemThemeChanged(_)
            | Message::CloseContextMenu
            | Message::ShowContextMenu { .. }
    )
}

#[cfg(target_os = "macos")]
fn cmd_pressed(modifiers: iced::keyboard::Modifiers) -> bool {
    modifiers.command()
}

#[cfg(not(target_os = "macos"))]
fn cmd_pressed(modifiers: iced::keyboard::Modifiers) -> bool {
    modifiers.control()
}

fn map_key_press_to_message(
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
    status: iced::event::Status,
) -> Option<Message> {
    use iced::keyboard::Key;
    use iced::keyboard::key::Named;

    if *key == Key::Named(Named::Escape) {
        return Some(Message::CloseModal);
    }

    let cmd = cmd_pressed(modifiers);

    if cmd && let Key::Character(character) = key {
        let lower = character.to_ascii_lowercase();
        match lower.as_str() {
            "k" => return Some(Message::FocusSearch),
            "r" => return Some(Message::RefreshEnvironment),
            "w" => return Some(Message::CloseWindow),
            _ => {}
        }

        if character.as_str() == "," {
            return Some(Message::NavigateToSettings);
        }
    }

    if !cmd
        && let Key::Character(character) = key
        && character.as_str() == "?"
    {
        return Some(Message::ShowKeyboardShortcuts);
    }

    if status == iced::event::Status::Captured
        && !cmd
        && !modifiers.alt()
        && let Key::Character(character) = key
    {
        let lower = character.to_ascii_lowercase();
        match lower.as_str() {
            "i" => return Some(Message::InstallHoveredVersionFromInput),
            "d" => return Some(Message::SetDefaultHoveredVersionFromInput),
            "u" => return Some(Message::UninstallHoveredVersionFromInput),
            _ => {}
        }
    }

    if status == iced::event::Status::Ignored
        && !cmd
        && !modifiers.alt()
        && let Key::Character(character) = key
    {
        let lower = character.to_ascii_lowercase();
        match lower.as_str() {
            "i" => return Some(Message::InstallHoveredVersion),
            "d" => return Some(Message::SetDefaultHoveredVersion),
            "u" => return Some(Message::UninstallHoveredVersion),
            _ => {}
        }
    }

    if *key == Key::Named(Named::Tab) {
        if cmd && modifiers.shift() {
            return Some(Message::SelectPreviousEnvironment);
        }
        if cmd {
            return Some(Message::SelectNextEnvironment);
        }
        if modifiers.shift() {
            return Some(Message::SelectPreviousVersion);
        }
        return Some(Message::SelectNextVersion);
    }

    match key {
        Key::Named(Named::ArrowUp) if status == iced::event::Status::Captured => {
            Some(Message::SelectPreviousVersionFromInput)
        }
        Key::Named(Named::ArrowDown) if status == iced::event::Status::Captured => {
            Some(Message::SelectNextVersionFromInput)
        }
        Key::Named(Named::ArrowUp) => Some(Message::SelectPreviousVersion),
        Key::Named(Named::ArrowDown) => Some(Message::SelectNextVersion),
        Key::Named(Named::Enter) => Some(Message::ActivateSelectedVersion),
        _ => None,
    }
}

fn keyboard_subscription() -> Subscription<Message> {
    iced::event::listen_with(|event, status, _id| {
        if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) =
            event
        {
            map_key_press_to_message(&key, modifiers, status)
        } else {
            None
        }
    })
}

fn window_events_subscription() -> Subscription<Message> {
    iced::event::listen_with(|event, _status, _id| {
        if let iced::Event::Window(window_event) = event {
            Some(Message::WindowEvent(window_event))
        } else {
            None
        }
    })
}

#[allow(clippy::struct_excessive_bools)]
pub struct Versi {
    pub(crate) state: AppState,
    pub(crate) settings: AppSettings,
    pub(crate) window_id: Option<iced::window::Id>,
    pub(crate) pending_minimize: bool,
    pub(crate) pending_show: bool,
    pub(crate) window_visible: bool,
    pub(crate) backend_path: PathBuf,
    pub(crate) backend_in_path: bool,
    pub(crate) backend_dir: Option<PathBuf>,
    pub(crate) window_size: Option<iced::Size>,
    pub(crate) window_position: Option<iced::Point>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) providers: HashMap<BackendKind, Arc<dyn BackendProvider>>,
    pub(crate) provider: Arc<dyn BackendProvider>,
    pub(crate) system_theme_mode: iced::theme::Mode,
}

impl Versi {
    pub fn new(settings: AppSettings) -> (Self, Task<Message>) {
        let should_minimize = settings.start_minimized
            && settings.tray_behavior != TrayBehavior::Disabled
            && tray::is_tray_active();

        let http_client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(settings.http_timeout_secs))
            .user_agent(format!("versi/{}", env!("CARGO_PKG_VERSION")))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                log::error!(
                    "Failed to build HTTP client with configured timeout ({}s): {error}. Falling back to default client settings.",
                    settings.http_timeout_secs
                );
                reqwest::Client::new()
            }
        };

        let fnm_provider: Arc<dyn BackendProvider> = Arc::new(versi_fnm::FnmProvider::new());
        let nvm_provider: Arc<dyn BackendProvider> = Arc::new(versi_nvm::NvmProvider::new());
        let volta_provider: Arc<dyn BackendProvider> =
            Arc::new(versi_volta::VoltaProvider::new(http_client.clone()));
        let asdf_provider: Arc<dyn BackendProvider> =
            Arc::new(versi_asdf::AsdfProvider::new(http_client.clone()));

        let mut providers: HashMap<BackendKind, Arc<dyn BackendProvider>> = HashMap::new();
        providers.insert(BackendKind::Fnm, fnm_provider.clone());
        providers.insert(BackendKind::Nvm, nvm_provider.clone());
        providers.insert(BackendKind::Asdf, asdf_provider);
        providers.insert(BackendKind::Volta, volta_provider);

        let preferred = settings.preferred_backend.unwrap_or(BackendKind::DEFAULT);
        let active_provider = providers.get(&preferred).cloned().unwrap_or(fnm_provider);

        let app = Self {
            state: AppState::Loading,
            settings,
            window_id: None,
            pending_minimize: should_minimize,
            pending_show: false,
            window_visible: !should_minimize,
            backend_path: PathBuf::from(active_provider.name()),
            backend_in_path: false,
            backend_dir: None,
            window_size: None,
            window_position: None,
            http_client,
            providers: providers.clone(),
            provider: active_provider,
            system_theme_mode: iced::theme::Mode::None,
        };

        let all_providers: Vec<Arc<dyn BackendProvider>> = providers.values().cloned().collect();
        let preferred_backend = app.settings.preferred_backend;
        let wsl_list_timeout = app.settings.wsl_list_timeout_secs;
        let wsl_distro_timeout = app.settings.wsl_distro_timeout_secs;
        let init_task = Task::perform(
            init::initialize(
                all_providers,
                preferred_backend,
                wsl_list_timeout,
                wsl_distro_timeout,
            ),
            |result| Message::Initialized(Box::new(result)),
        );
        let theme_task = iced::system::theme().map(Message::SystemThemeChanged);
        let tray_task = Task::done(Message::InitTray);

        (app, Task::batch([init_task, theme_task, tray_task]))
    }

    pub fn title(&self) -> String {
        match &self.state {
            AppState::Loading => "Versi".to_string(),
            AppState::Onboarding(_) => "Versi - Setup".to_string(),
            AppState::Main(state) => {
                if let Some(v) = &state.active_environment().default_version {
                    format!("Versi - Node {v}")
                } else {
                    "Versi".to_string()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            AppState::Loading => views::loading::view(),
            AppState::Onboarding(state) => {
                let backend_name = state.selected_backend.unwrap_or(self.active_backend_kind());
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
                    let tabs_container = container(tabs).padding(
                        iced::Padding::new(0.0)
                            .top(12.0)
                            .left(crate::theme::tokens::INSET_RIGHT)
                            .right(crate::theme::tokens::INSET_RIGHT),
                    );
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
                if tray::is_tray_active() {
                    TICK_INTERVAL_FAST_MS
                } else {
                    TICK_INTERVAL_DEFAULT_MS
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                TICK_INTERVAL_DEFAULT_MS
            }
        };
        let tick =
            iced::time::every(std::time::Duration::from_millis(tick_ms)).map(|_| Message::Tick);

        let keyboard = keyboard_subscription();
        let window_events = window_events_subscription();

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

    fn handle_preferred_backend_changed(&mut self, name: BackendKind) -> Task<Message> {
        self.settings.preferred_backend = Some(name);
        self.save_settings_with_log();

        if let AppState::Main(state) = &mut self.state {
            let is_detected = state.detected_backends.contains(&name);
            if is_detected && state.backend_name != name {
                if let Some(provider) = self.providers.get(&name) {
                    self.provider = provider.clone();
                }
                let all_providers = self.all_providers();
                let preferred = self.settings.preferred_backend;
                let wsl_list_timeout = self.settings.wsl_list_timeout_secs;
                let wsl_distro_timeout = self.settings.wsl_distro_timeout_secs;
                self.state = AppState::Loading;
                return Task::perform(
                    init::initialize(
                        all_providers,
                        preferred,
                        wsl_list_timeout,
                        wsl_distro_timeout,
                    ),
                    |result| Message::Initialized(Box::new(result)),
                );
            }
        }

        Task::none()
    }

    pub(crate) fn all_providers(&self) -> Vec<Arc<dyn BackendProvider>> {
        self.providers.values().cloned().collect()
    }

    pub(crate) fn provider_for_kind(&self, kind: BackendKind) -> Arc<dyn BackendProvider> {
        self.providers
            .get(&kind)
            .cloned()
            .unwrap_or_else(|| self.provider.clone())
    }

    pub(crate) fn active_provider(&self) -> Arc<dyn BackendProvider> {
        if let AppState::Main(state) = &self.state {
            self.provider_for_kind(state.backend_name)
        } else {
            self.provider.clone()
        }
    }

    pub(crate) fn active_backend_kind(&self) -> BackendKind {
        if let AppState::Main(state) = &self.state {
            state.backend_name
        } else {
            BackendKind::from_name(self.provider.name()).unwrap_or(BackendKind::DEFAULT)
        }
    }
}

#[cfg(test)]
fn test_app_with_two_environments() -> Versi {
    let fnm_provider: Arc<dyn BackendProvider> = Arc::new(versi_fnm::FnmProvider::new());
    let nvm_provider: Arc<dyn BackendProvider> = Arc::new(versi_nvm::NvmProvider::new());
    let asdf_provider: Arc<dyn BackendProvider> =
        Arc::new(versi_asdf::AsdfProvider::new(reqwest::Client::new()));
    let volta_provider: Arc<dyn BackendProvider> =
        Arc::new(versi_volta::VoltaProvider::new(reqwest::Client::new()));

    let mut providers: HashMap<BackendKind, Arc<dyn BackendProvider>> = HashMap::new();
    providers.insert(BackendKind::Fnm, fnm_provider.clone());
    providers.insert(BackendKind::Nvm, nvm_provider.clone());
    providers.insert(BackendKind::Asdf, asdf_provider);
    providers.insert(BackendKind::Volta, volta_provider);

    let detection = BackendDetection {
        found: true,
        path: Some(PathBuf::from("fnm")),
        version: None,
        in_path: true,
        data_dir: None,
    };
    let backend = fnm_provider.create_manager(&detection);

    let native = EnvironmentState::new(EnvironmentId::Native, BackendKind::Fnm, None);
    let wsl = EnvironmentState::new(
        EnvironmentId::Wsl {
            distro: "Ubuntu".to_string(),
            backend_path: "/home/user/.nvm/nvm.sh".to_string(),
        },
        BackendKind::Nvm,
        None,
    );
    let main_state = MainState::new_with_environments(backend, vec![native, wsl], BackendKind::Fnm);

    Versi {
        state: AppState::Main(Box::new(main_state)),
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

#[cfg(test)]
impl Versi {
    fn main_state(&self) -> &MainState {
        match &self.state {
            AppState::Main(state) => state,
            other => panic!("expected Main state, got {other:?}"),
        }
    }

    fn main_state_mut(&mut self) -> &mut MainState {
        match &mut self.state {
            AppState::Main(state) => state,
            other => panic!("expected Main state, got {other:?}"),
        }
    }

    fn onboarding_state(&self) -> &OnboardingState {
        match &self.state {
            AppState::Onboarding(state) => state,
            other => panic!("expected Onboarding state, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion};
    use versi_platform::EnvironmentId;

    use super::{
        map_key_press_to_message, should_dismiss_context_menu, test_app_with_two_environments,
    };
    use crate::backend_kind::BackendKind;
    use crate::error::AppError;
    use crate::message::Message;
    use crate::state::{MainViewKind, Modal, Operation};
    use crate::tray::TrayMessage;

    #[test]
    fn context_menu_is_dismissed_for_unrelated_messages() {
        assert!(should_dismiss_context_menu(&Message::NavigateToSettings));
        assert!(should_dismiss_context_menu(&Message::SetDefault(
            "20.10.0".to_string()
        )));
    }

    #[test]
    fn context_menu_stays_open_for_allowed_messages() {
        assert!(!should_dismiss_context_menu(&Message::Tick));
        assert!(!should_dismiss_context_menu(&Message::ShowContextMenu {
            version: "20.10.0".to_string(),
            is_installed: true,
            is_default: false,
        }));
    }

    #[test]
    fn tab_shortcuts_navigate_versions() {
        use iced::event::Status;
        use iced::keyboard::Key;
        use iced::keyboard::Modifiers;
        use iced::keyboard::key::Named;

        let message =
            map_key_press_to_message(&Key::Named(Named::Tab), Modifiers::empty(), Status::Ignored);
        assert!(matches!(message, Some(Message::SelectNextVersion)));

        let mut modifiers = Modifiers::empty();
        modifiers.insert(Modifiers::SHIFT);
        let shifted = map_key_press_to_message(&Key::Named(Named::Tab), modifiers, Status::Ignored);
        assert!(matches!(shifted, Some(Message::SelectPreviousVersion)));
    }

    #[test]
    fn arrow_shortcuts_navigate_versions() {
        use iced::event::Status;
        use iced::keyboard::Key;
        use iced::keyboard::Modifiers;
        use iced::keyboard::key::Named;

        let next = map_key_press_to_message(
            &Key::Named(Named::ArrowDown),
            Modifiers::empty(),
            Status::Ignored,
        );
        let previous = map_key_press_to_message(
            &Key::Named(Named::ArrowUp),
            Modifiers::empty(),
            Status::Ignored,
        );
        assert!(matches!(next, Some(Message::SelectNextVersion)));
        assert!(matches!(previous, Some(Message::SelectPreviousVersion)));
    }

    #[test]
    fn hovered_actions_map_when_status_ignored() {
        use iced::event::Status;
        use iced::keyboard::Key;
        use iced::keyboard::Modifiers;

        let install = map_key_press_to_message(
            &Key::Character("i".into()),
            Modifiers::empty(),
            Status::Ignored,
        );
        let set_default = map_key_press_to_message(
            &Key::Character("D".into()),
            Modifiers::empty(),
            Status::Ignored,
        );
        let uninstall = map_key_press_to_message(
            &Key::Character("u".into()),
            Modifiers::empty(),
            Status::Ignored,
        );

        assert!(matches!(install, Some(Message::InstallHoveredVersion)));
        assert!(matches!(
            set_default,
            Some(Message::SetDefaultHoveredVersion)
        ));
        assert!(matches!(uninstall, Some(Message::UninstallHoveredVersion)));
    }

    #[test]
    fn hovered_actions_from_input_map_when_event_is_captured() {
        use iced::event::Status;
        use iced::keyboard::Key;
        use iced::keyboard::Modifiers;

        let install = map_key_press_to_message(
            &Key::Character("i".into()),
            Modifiers::empty(),
            Status::Captured,
        );
        let set_default = map_key_press_to_message(
            &Key::Character("d".into()),
            Modifiers::empty(),
            Status::Captured,
        );
        let uninstall = map_key_press_to_message(
            &Key::Character("u".into()),
            Modifiers::empty(),
            Status::Captured,
        );

        assert!(matches!(
            install,
            Some(Message::InstallHoveredVersionFromInput)
        ));
        assert!(matches!(
            set_default,
            Some(Message::SetDefaultHoveredVersionFromInput)
        ));
        assert!(matches!(
            uninstall,
            Some(Message::UninstallHoveredVersionFromInput)
        ));
    }

    #[test]
    fn command_hovered_actions_do_not_shadow_other_shortcuts() {
        use iced::event::Status;
        use iced::keyboard::Key;
        use iced::keyboard::Modifiers;

        let mut modifiers = Modifiers::empty();
        #[cfg(target_os = "macos")]
        modifiers.insert(Modifiers::LOGO);
        #[cfg(not(target_os = "macos"))]
        modifiers.insert(Modifiers::CTRL);

        let install =
            map_key_press_to_message(&Key::Character("i".into()), modifiers, Status::Captured);
        let set_default =
            map_key_press_to_message(&Key::Character("d".into()), modifiers, Status::Captured);
        let uninstall =
            map_key_press_to_message(&Key::Character("u".into()), modifiers, Status::Captured);

        assert!(install.is_none());
        assert!(set_default.is_none());
        assert!(uninstall.is_none());
    }

    #[test]
    fn arrow_shortcuts_from_captured_state_blur_search_path() {
        use iced::event::Status;
        use iced::keyboard::Key;
        use iced::keyboard::Modifiers;
        use iced::keyboard::key::Named;

        let next = map_key_press_to_message(
            &Key::Named(Named::ArrowDown),
            Modifiers::empty(),
            Status::Captured,
        );
        let previous = map_key_press_to_message(
            &Key::Named(Named::ArrowUp),
            Modifiers::empty(),
            Status::Captured,
        );

        assert!(matches!(next, Some(Message::SelectNextVersionFromInput)));
        assert!(matches!(
            previous,
            Some(Message::SelectPreviousVersionFromInput)
        ));
    }

    #[test]
    fn environment_switch_updates_active_backend_kind_and_provider() {
        let mut app = test_app_with_two_environments();

        let _ = app.handle_environment_selected(1);

        let state = app.main_state();
        assert_eq!(state.active_environment_idx, 1);
        assert_eq!(state.backend_name, BackendKind::Nvm);
        assert_eq!(app.provider.name(), BackendKind::Nvm.as_str());
    }

    #[test]
    fn tray_set_default_switches_environment_before_queueing_operation() {
        let mut app = test_app_with_two_environments();
        let target_env_id = app.main_state().environments[1].id.clone();

        let _ = app.handle_tray_event(TrayMessage::SetDefault {
            env_id: target_env_id,
            version: "20.11.0".to_string(),
        });

        let state = app.main_state();
        assert_eq!(state.active_environment_idx, 1);
        assert_eq!(state.backend_name, BackendKind::Nvm);
        assert_eq!(app.provider.name(), BackendKind::Nvm.as_str());
        assert!(matches!(
            state.operation_queue.exclusive_op,
            Some(Operation::SetDefault { ref version }) if version == "20.11.0"
        ));
    }

    #[test]
    fn environment_load_failure_sets_error_on_target_environment() {
        let mut app = test_app_with_two_environments();
        let target_env = EnvironmentId::Wsl {
            distro: "Ubuntu".to_string(),
            backend_path: "/home/user/.nvm/nvm.sh".to_string(),
        };

        let _ = app.handle_environment_loaded(
            &target_env,
            0,
            Err(AppError::environment_load_failed("backend unavailable")),
        );

        let state = app.main_state();

        let failed_env = state
            .environments
            .iter()
            .find(|env| env.id == target_env)
            .expect("expected target environment");
        assert!(!failed_env.loading);
        assert_eq!(
            failed_env.error,
            Some(AppError::environment_load_failed("backend unavailable"))
        );

        let native_env = state
            .environments
            .iter()
            .find(|env| env.id == EnvironmentId::Native)
            .expect("expected native environment");
        assert!(native_env.error.is_none());
    }

    #[test]
    fn environment_load_success_clears_previous_error() {
        let mut app = test_app_with_two_environments();
        let target_env = EnvironmentId::Native;

        let _ = app.handle_environment_loaded(
            &target_env,
            0,
            Err(AppError::environment_load_failed("timed out")),
        );
        let _ = app.handle_environment_loaded(
            &target_env,
            0,
            Ok(vec![InstalledVersion {
                version: NodeVersion::new(20, 11, 0),
                is_default: true,
                lts_codename: None,
                install_date: None,
                disk_size: None,
            }]),
        );

        let state = app.main_state();
        let env = state
            .environments
            .iter()
            .find(|env| env.id == target_env)
            .expect("expected native environment");
        assert!(!env.loading);
        assert!(env.error.is_none());
        assert_eq!(env.default_version, Some(NodeVersion::new(20, 11, 0)));
        assert!(env.installed_set.contains(&NodeVersion::new(20, 11, 0)));
    }

    #[test]
    fn stale_environment_load_response_is_ignored() {
        let mut app = test_app_with_two_environments();
        let target_env = EnvironmentId::Native;

        let env = app
            .main_state_mut()
            .environments
            .iter_mut()
            .find(|env| env.id == target_env)
            .expect("expected native environment");
        env.loading = true;
        env.load_request_seq = 2;

        let _ = app.handle_environment_loaded(
            &target_env,
            1,
            Ok(vec![InstalledVersion {
                version: NodeVersion::new(20, 11, 0),
                is_default: true,
                lts_codename: None,
                install_date: None,
                disk_size: None,
            }]),
        );

        let state = app.main_state();
        let env = state
            .environments
            .iter()
            .find(|env| env.id == EnvironmentId::Native)
            .expect("expected native environment");
        assert!(env.loading);
        assert!(env.installed_versions.is_empty());
        assert!(env.default_version.is_none());
    }

    #[test]
    fn stale_remote_versions_response_is_ignored() {
        let mut app = test_app_with_two_environments();

        app.main_state_mut().available_versions.loading = true;
        app.main_state_mut().available_versions.remote.request_seq = 2;

        app.handle_remote_versions_fetched(
            1,
            Ok(vec![RemoteVersion {
                version: NodeVersion::new(22, 1, 0),
                lts_codename: None,
                is_latest: true,
            }]),
        );

        let state = app.main_state();
        assert!(state.available_versions.loading);
        assert!(state.available_versions.versions.is_empty());
    }

    #[test]
    fn update_routes_navigation_messages() {
        let mut app = test_app_with_two_environments();

        let _ = app.update(Message::SearchChanged("20".to_string()));

        let state = app.main_state();
        assert_eq!(state.search_query, "20");
    }

    #[test]
    fn update_routes_operation_messages() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut().modal = Some(Modal::KeyboardShortcuts);

        let _ = app.update(Message::CancelBulkOperation);

        let state = app.main_state();
        assert!(state.modal.is_none());
    }

    #[test]
    fn update_routes_settings_messages() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut().view = MainViewKind::Versions;

        let _ = app.update(Message::NavigateToAbout);

        let state = app.main_state();
        assert_eq!(state.view, MainViewKind::About);
    }

    #[test]
    fn update_routes_system_messages() {
        let mut app = test_app_with_two_environments();
        let point = iced::Point::new(12.0, 34.0);

        let _ = app.update(Message::VersionListCursorMoved(point));

        let state = app.main_state();
        assert_eq!(state.cursor_position, point);
    }
}
