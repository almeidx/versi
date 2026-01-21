use iced::window;

mod app;
mod message;
mod settings;
mod state;
mod theme;
mod views;
mod widgets;

fn main() -> iced::Result {
    iced::application(app::FnmUi::new, app::FnmUi::update, app::FnmUi::view)
        .title(|state: &app::FnmUi| state.title())
        .subscription(|state: &app::FnmUi| state.subscription())
        .theme(|state: &app::FnmUi| state.theme())
        .window(window::Settings {
            size: iced::Size::new(800.0, 600.0),
            min_size: Some(iced::Size::new(600.0, 400.0)),
            ..Default::default()
        })
        .run()
}
