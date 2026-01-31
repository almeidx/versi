use std::collections::HashSet;

use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Alignment, Element, Length};

use versi_backend::RemoteVersion;
use versi_core::ReleaseSchedule;

use crate::icon;
use crate::message::Message;
use crate::state::OperationQueue;
use crate::theme::styles;

pub(super) fn available_version_row<'a>(
    version: &'a RemoteVersion,
    schedule: Option<&ReleaseSchedule>,
    operation_queue: &'a OperationQueue,
    installed_set: &HashSet<String>,
    hovered_version: &'a Option<String>,
) -> Element<'a, Message> {
    let version_str = version.version.to_string();
    let is_eol = schedule
        .map(|s| !s.is_active(version.version.major))
        .unwrap_or(false);
    let version_display = version_str.clone();
    let version_for_changelog = version_str.clone();
    let version_for_hover = version_str.clone();
    let is_installed = installed_set.contains(&version_str);

    let is_active = operation_queue.is_current_version(&version_str);
    let is_pending = operation_queue.has_pending_for_version(&version_str);
    let is_button_hovered = hovered_version.as_ref().is_some_and(|h| h == &version_str);

    let action_button: Element<Message> = if is_active {
        button(text("Installing...").size(12))
            .style(styles::primary_button)
            .padding([6, 12])
            .into()
    } else if is_pending {
        button(text("Queued").size(12))
            .style(styles::secondary_button)
            .padding([6, 12])
            .into()
    } else if is_installed {
        let btn = if is_button_hovered {
            button(text("Uninstall").size(12))
                .on_press(Message::RequestUninstall(version_str))
                .style(styles::danger_button)
                .padding([6, 12])
        } else {
            button(text("Installed").size(12))
                .style(styles::secondary_button)
                .padding([6, 12])
        };
        mouse_area(btn)
            .on_enter(Message::VersionRowHovered(Some(version_for_hover)))
            .on_exit(Message::VersionRowHovered(None))
            .into()
    } else {
        button(text("Install").size(12))
            .on_press(Message::StartInstall(version_str))
            .style(styles::primary_button)
            .padding([6, 12])
            .into()
    };

    row![
        text(version_display).size(14).width(Length::Fixed(120.0)),
        if let Some(lts) = &version.lts_codename {
            container(text(format!("LTS: {}", lts)).size(11))
                .padding([2, 6])
                .style(styles::badge_lts)
        } else {
            container(Space::new())
        },
        if is_eol {
            container(text("End-of-Life").size(11))
                .padding([2, 6])
                .style(styles::badge_eol)
        } else {
            container(Space::new())
        },
        Space::new().width(Length::Fill),
        button(
            row![text("Changelog").size(11), icon::arrow_up_right(11.0),]
                .spacing(2)
                .align_y(Alignment::Center),
        )
        .on_press(Message::OpenChangelog(version_for_changelog))
        .style(styles::ghost_button)
        .padding([4, 8]),
        action_button,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([4, 8])
    .into()
}
