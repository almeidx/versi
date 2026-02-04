#![windows_subsystem = "windows"]

use iced::window;

mod app;
mod cache;
mod icon;
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
    logging::init_logging(settings.debug_logging, settings.max_log_size_bytes);

    log::info!("Versi {} starting", env!("CARGO_PKG_VERSION"));

    #[cfg(target_os = "linux")]
    {
        if let Err(e) = gtk::init() {
            log::warn!("Failed to initialize GTK: {}", e);
        }
    }

    if let Err(e) = tray::init_tray(&settings.tray_behavior) {
        log::warn!("Failed to initialize tray icon: {}", e);
    }

    let icon = window::icon::from_file_data(include_bytes!("../../../assets/logo.png"), None).ok();

    let (window_size, window_position) = match &settings.window_geometry {
        Some(geo) if geo.is_likely_visible() => (
            iced::Size::new(geo.width, geo.height),
            window::Position::Specific(iced::Point::new(geo.x as f32, geo.y as f32)),
        ),
        _ => (iced::Size::new(800.0, 600.0), window::Position::Default),
    };

    iced::application(app::Versi::new, app::Versi::update, app::Versi::view)
        .title(|state: &app::Versi| state.title())
        .subscription(|state: &app::Versi| state.subscription())
        .theme(|state: &app::Versi| state.theme())
        .window(window::Settings {
            size: window_size,
            position: window_position,
            min_size: Some(iced::Size::new(600.0, 400.0)),
            icon,
            visible: true,
            exit_on_close_request: false,
            ..Default::default()
        })
        .run()
}
