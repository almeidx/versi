use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Alignment, Element, Length};

use crate::icon;
use crate::message::Message;
use crate::state::{AppUpdateState, MainState};
use crate::theme::styles;
use crate::widgets::helpers::nav_icons;

pub(super) fn header_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    let env = state.active_environment();

    let subtitle = match &env.backend_version {
        Some(v) => format!("{} {}", state.backend_name, v),
        None => state.backend_name.to_string(),
    };

    let mut left = row![text(subtitle).size(14),]
        .spacing(8)
        .align_y(Alignment::Center);

    if let Some(update) = &state.app_update {
        left = left.push(app_update_badge(update, &state.app_update_state));
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

    row![
        left,
        Space::new().width(Length::Fill),
        nav_icons(&state.view, state.refresh_rotation),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn app_update_badge<'a>(
    update: &versi_core::AppUpdate,
    update_state: &AppUpdateState,
) -> Element<'a, Message> {
    let mut badge_row = row![].spacing(4).align_y(Alignment::Center);

    match update_state {
        AppUpdateState::Idle => {
            let main_btn = button(
                container(
                    row![text(format!("v{} available — Update", update.latest_version)).size(11),]
                        .spacing(2)
                        .align_y(Alignment::Center),
                )
                .padding([2, 8]),
            )
            .style(styles::app_update_button)
            .padding(0);

            let main_btn = if update.download_url.is_some() {
                main_btn.on_press(Message::StartAppUpdate)
            } else {
                main_btn.on_press(Message::OpenAppUpdate)
            };

            badge_row = badge_row.push(main_btn);

            if update.download_url.is_some() {
                badge_row = badge_row.push(
                    button(container(icon::arrow_up_right(11.0)).padding([2, 4]))
                        .on_press(Message::OpenAppUpdate)
                        .style(styles::app_update_button)
                        .padding(0),
                );
            }
        }
        AppUpdateState::Downloading { downloaded, total } => {
            let label = if *total > 0 {
                let pct = (*downloaded as f64 / *total as f64 * 100.0) as u32;
                format!("Updating {}%", pct)
            } else {
                "Updating...".to_string()
            };
            badge_row = badge_row.push(
                button(container(text(label).size(11)).padding([2, 8]))
                    .style(styles::app_update_button)
                    .padding(0),
            );
        }
        AppUpdateState::Extracting => {
            badge_row = badge_row.push(
                button(container(text("Extracting...").size(11)).padding([2, 8]))
                    .style(styles::app_update_button)
                    .padding(0),
            );
        }
        AppUpdateState::Applying => {
            badge_row = badge_row.push(
                button(container(text("Applying...").size(11)).padding([2, 8]))
                    .style(styles::app_update_button)
                    .padding(0),
            );
        }
        AppUpdateState::RestartRequired => {
            badge_row = badge_row.push(
                button(container(text("Restart to update").size(11)).padding([2, 8]))
                    .on_press(Message::RestartApp)
                    .style(styles::app_update_button)
                    .padding(0),
            );
        }
        AppUpdateState::Failed(err) => {
            badge_row = badge_row.push(
                tooltip(
                    button(container(text("Update failed — Retry").size(11)).padding([2, 8]))
                        .on_press(Message::StartAppUpdate)
                        .style(styles::app_update_button)
                        .padding(0),
                    container(text(err.clone()).size(12))
                        .padding([4, 8])
                        .style(styles::tooltip_container),
                    tooltip::Position::Bottom,
                )
                .gap(4.0),
            );

            badge_row = badge_row.push(
                button(container(icon::arrow_up_right(11.0)).padding([2, 4]))
                    .on_press(Message::OpenAppUpdate)
                    .style(styles::app_update_button)
                    .padding(0),
            );
        }
    }

    badge_row.into()
}
