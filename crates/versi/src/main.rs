#![windows_subsystem = "windows"]

use iced::window;

mod app;
mod backend_kind;
mod cache;
mod error;
mod format;
mod fs_utils;
mod icon;
mod logging;
mod message;
mod settings;
mod single_instance;
mod state;
mod theme;
mod tray;
mod version_query;
mod views;
mod widgets;
#[cfg(windows)]
mod windows_window;

fn main() -> iced::Result {
    let _instance_guard = match single_instance::SingleInstance::acquire() {
        Ok(guard) => guard,
        Err(single_instance::AcquireError::AlreadyRunning) => {
            single_instance::bring_existing_window_to_front();
            return Ok(());
        }
        Err(error) => {
            eprintln!("Error: failed to acquire single-instance lock: {error}");
            std::process::exit(1);
        }
    };

    if let Err(e) = versi_platform::AppPaths::new() {
        eprintln!(
            "Error: {e}. Versi cannot determine where to store its data. Please ensure your system environment is configured correctly."
        );
        std::process::exit(1);
    }

    let settings = settings::AppSettings::load();
    logging::init_logging(settings.debug_logging, settings.max_log_size_bytes);

    log::info!("Versi {} starting", env!("CARGO_PKG_VERSION"));

    #[cfg(windows)]
    if std::env::var_os("WGPU_POWER_PREF").is_none() {
        // SAFETY: no other threads exist yet; the tray and iced threads start later.
        unsafe { std::env::set_var("WGPU_POWER_PREF", "low") };
    }

    #[cfg(target_os = "linux")]
    {
        if let Err(e) = gtk::init() {
            log::warn!("Failed to initialize GTK: {e}");
        }
    }

    let icon = window::icon::from_file_data(include_bytes!("../../../assets/logo.png"), None).ok();

    let (window_size, window_position) = match &settings.window_geometry {
        Some(geo) if geo.is_likely_visible() => (
            iced::Size::new(geo.width, geo.height),
            window::Position::Specific(iced::Point::new(geo.x, geo.y)),
        ),
        _ => (iced::Size::new(800.0, 600.0), window::Position::Default),
    };

    #[cfg(target_os = "linux")]
    let platform_specific = window::settings::PlatformSpecific {
        application_id: versi_platform::APP_ID.to_string(),
        ..Default::default()
    };
    #[cfg(not(target_os = "linux"))]
    let platform_specific = window::settings::PlatformSpecific::default();

    iced::application(
        move || app::Versi::new(settings.clone()),
        app::Versi::update,
        app::Versi::view,
    )
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
        platform_specific,
        ..Default::default()
    })
    .run()
}
