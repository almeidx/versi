use iced::widget::{button, container, row, text, tooltip};
use iced::{Alignment, Element};

use crate::icon;
use crate::message::Message;
use crate::state::MainViewKind;
use crate::theme::styles;

pub fn styled_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    label: &'a str,
    position: tooltip::Position,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(label).size(12))
            .padding([4, 8])
            .style(styles::tooltip_container),
        position,
    )
    .gap(4.0)
    .into()
}

pub fn nav_icons<'a>(active_view: &MainViewKind, refresh_rotation: f32) -> Element<'a, Message> {
    let refresh_icon = if refresh_rotation != 0.0 {
        icon::refresh_spinning(16.0, refresh_rotation)
    } else {
        icon::refresh(16.0)
    };

    let settings_style = if *active_view == MainViewKind::Settings {
        styles::ghost_button_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        styles::ghost_button
    };

    let about_style = if *active_view == MainViewKind::About {
        styles::ghost_button_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        styles::ghost_button
    };

    let home_style = if *active_view == MainViewKind::Versions {
        styles::ghost_button_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        styles::ghost_button
    };

    row![
        styled_tooltip(
            button(refresh_icon)
                .on_press(Message::RefreshEnvironment)
                .style(styles::ghost_button)
                .padding([4, 6]),
            "Refresh",
            tooltip::Position::Bottom,
        ),
        styled_tooltip(
            button(icon::home(16.0))
                .on_press(Message::NavigateToVersions)
                .style(home_style)
                .padding([4, 6]),
            "Home",
            tooltip::Position::Bottom,
        ),
        styled_tooltip(
            button(icon::settings(16.0))
                .on_press(Message::NavigateToSettings)
                .style(settings_style)
                .padding([4, 6]),
            "Settings",
            tooltip::Position::Bottom,
        ),
        styled_tooltip(
            button(icon::info(16.0))
                .on_press(Message::NavigateToAbout)
                .style(about_style)
                .padding([4, 6]),
            "About",
            tooltip::Position::Bottom,
        ),
    ]
    .spacing(2)
    .align_y(Alignment::Center)
    .into()
}
