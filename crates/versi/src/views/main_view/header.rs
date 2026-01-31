use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Alignment, Element, Length};

use crate::icon;
use crate::message::Message;
use crate::state::MainState;
use crate::theme::styles;

pub(super) fn header_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    let env = state.active_environment();

    let subtitle = match &env.backend_version {
        Some(v) => format!("{} {}", state.backend_name, v),
        None => state.backend_name.to_string(),
    };

    let mut left = row![
        text(subtitle)
            .size(12)
            .color(iced::Color::from_rgb8(142, 142, 147)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if let Some(update) = &state.app_update {
        left = left.push(
            button(
                container(
                    row![
                        text(format!("v{} available", update.latest_version)).size(11),
                        icon::arrow_up_right(11.0),
                    ]
                    .spacing(2)
                    .align_y(Alignment::Center),
                )
                .padding([2, 8]),
            )
            .on_press(Message::OpenAppUpdate)
            .style(styles::app_update_button)
            .padding(0),
        );
    }

    if let Some(update) = &state.backend_update {
        left = left.push(
            button(
                container(
                    row![
                        text(format!(
                            "{} {} available",
                            state.backend_name, update.latest_version
                        ))
                        .size(11),
                        icon::arrow_up_right(11.0),
                    ]
                    .spacing(2)
                    .align_y(Alignment::Center),
                )
                .padding([2, 8]),
            )
            .on_press(Message::OpenBackendUpdate)
            .style(styles::app_update_button)
            .padding(0),
        );
    }

    let refresh_icon = if state.refresh_rotation != 0.0 {
        icon::refresh_spinning(16.0, state.refresh_rotation)
    } else {
        icon::refresh(16.0)
    };

    let icon_row = row![
        tooltip(
            button(refresh_icon)
                .on_press(Message::RefreshEnvironment)
                .style(styles::ghost_button)
                .padding([4, 6]),
            text("Refresh").size(12),
            tooltip::Position::Bottom,
        ),
        tooltip(
            button(icon::settings(16.0))
                .on_press(Message::NavigateToSettings)
                .style(styles::ghost_button)
                .padding([4, 6]),
            text("Settings").size(12),
            tooltip::Position::Bottom,
        ),
        tooltip(
            button(icon::info(16.0))
                .on_press(Message::NavigateToAbout)
                .style(styles::ghost_button)
                .padding([4, 6]),
            text("About").size(12),
            tooltip::Position::Bottom,
        ),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    row![left, Space::new().width(Length::Fill), icon_row]
        .align_y(Alignment::Center)
        .into()
}
