#![windows_subsystem = "windows"]

use iced::window;

mod app;
mod logging;
mod message;
mod settings;
mod single_instance;
mod state;
mod theme;
mod tray;
mod views;
mod widgets;

fn main() -> iced::Result {
    let _instance_guard = match single_instance::SingleInstance::acquire() {
        Ok(guard) => guard,
        Err(_) => {
            single_instance::bring_existing_window_to_front();
            return Ok(());
        }
    };

    let settings = settings::AppSettings::load();
    logging::init_logging(settings.debug_logging);

    log::info!("Versi {} starting", env!("CARGO_PKG_VERSION"));

    if let Err(e) = tray::init_tray(&settings.tray_behavior) {
        log::warn!("Failed to initialize tray icon: {}", e);
    }

    let icon = window::icon::from_file_data(include_bytes!("../../../assets/logo.png"), None).ok();

    let visible =
        !settings.start_minimized || settings.tray_behavior == settings::TrayBehavior::Disabled;

    iced::application(app::FnmUi::new, app::FnmUi::update, app::FnmUi::view)
        .title(|state: &app::FnmUi| state.title())
        .subscription(|state: &app::FnmUi| state.subscription())
        .theme(|state: &app::FnmUi| state.theme())
        .window(window::Settings {
            size: iced::Size::new(800.0, 600.0),
            min_size: Some(iced::Size::new(600.0, 400.0)),
            icon,
            visible,
            ..Default::default()
        })
        .run()
}
