use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use versi_backend::{InstalledVersion, VersionGroup};
use versi_core::ReleaseSchedule;

use crate::icon;
use crate::message::Message;
use crate::state::OperationQueue;
use crate::theme::styles;

use super::filter_version;
use super::item::version_item_view;

pub(super) fn version_group_view<'a>(
    group: &'a VersionGroup,
    default: &'a Option<versi_backend::NodeVersion>,
    search_query: &'a str,
    update_available: Option<String>,
    schedule: Option<&ReleaseSchedule>,
    operation_queue: &'a OperationQueue,
    hovered_version: &'a Option<String>,
) -> Element<'a, Message> {
    let has_lts = group.versions.iter().any(|v| v.lts_codename.is_some());
    let has_default = group
        .versions
        .iter()
        .any(|v| default.as_ref().map(|d| d == &v.version).unwrap_or(false));
    let is_eol = schedule.map(|s| !s.is_active(group.major)).unwrap_or(false);

    let chevron = if group.is_expanded {
        icon::chevron_down(12.0)
    } else {
        icon::chevron_right(12.0)
    };

    let mut header_row = row![
        chevron,
        text(format!("Node {}.x", group.major)).size(16),
        text(format!("({} installed)", group.versions.len())).size(12),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if has_lts {
        header_row = header_row.push(
            container(text("LTS").size(10))
                .padding([2, 6])
                .style(styles::badge_lts),
        );
    }

    if is_eol {
        header_row = header_row.push(
            container(text("End-of-Life").size(10))
                .padding([2, 6])
                .style(styles::badge_eol),
        );
    }

    if has_default && !group.is_expanded {
        header_row = header_row.push(
            container(text("default").size(10))
                .padding([2, 6])
                .style(styles::badge_default),
        );
    }

    let header_button = button(header_row)
        .on_press(Message::VersionGroupToggled { major: group.major })
        .style(|theme, status| {
            let mut style = iced::widget::button::text(theme, status);
            style.text_color = theme.palette().text;
            style
        })
        .padding([8, 12]);

    let mut header_actions = row![].spacing(8).align_y(Alignment::Center);

    if let Some(new_version) = update_available {
        let version_to_install = new_version.clone();
        header_actions = header_actions.push(
            button(container(text(format!("{} available", new_version)).size(10)).padding([2, 6]))
                .on_press(Message::StartInstall(version_to_install))
                .style(styles::update_badge_button)
                .padding([0, 4]),
        );
    }

    if group.is_expanded && group.versions.len() > 1 {
        header_actions = header_actions.push(
            button(text("Keep Latest").size(10))
                .on_press(Message::RequestBulkUninstallMajorExceptLatest { major: group.major })
                .style(styles::ghost_button)
                .padding([4, 8]),
        );
        header_actions = header_actions.push(
            button(text("Uninstall All").size(10))
                .on_press(Message::RequestBulkUninstallMajor { major: group.major })
                .style(styles::ghost_button)
                .padding([4, 8]),
        );
    }

    let header: Element<Message> = row![
        header_button,
        Space::new().width(Length::Fill),
        header_actions,
    ]
    .align_y(Alignment::Center)
    .into();

    if group.is_expanded {
        let filtered_versions: Vec<&InstalledVersion> = group
            .versions
            .iter()
            .filter(|v| filter_version(v, search_query))
            .collect();

        let items: Vec<Element<Message>> = filtered_versions
            .iter()
            .map(|v| version_item_view(v, default, operation_queue, hovered_version))
            .collect();

        container(
            column![
                header,
                container(column(items).spacing(2)).padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 24.0,
                }),
            ]
            .spacing(4),
        )
        .style(styles::card_container)
        .padding(12)
        .into()
    } else {
        container(header)
            .style(styles::card_container)
            .padding(12)
            .width(Length::Fill)
            .into()
    }
}
