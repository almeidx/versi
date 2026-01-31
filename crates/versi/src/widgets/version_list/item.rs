use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Alignment, Element, Length};

use versi_backend::InstalledVersion;

use crate::icon;
use crate::message::Message;
use crate::state::{Operation, OperationQueue};
use crate::theme::styles;

pub(super) fn version_item_view<'a>(
    version: &'a InstalledVersion,
    default: &'a Option<versi_backend::NodeVersion>,
    operation_queue: &'a OperationQueue,
    hovered_version: &'a Option<String>,
) -> Element<'a, Message> {
    let is_default = default
        .as_ref()
        .map(|d| d == &version.version)
        .unwrap_or(false);

    let version_str = version.version.to_string();
    let version_display = version_str.clone();
    let version_for_default = version_str.clone();
    let version_for_changelog = version_str.clone();
    let version_for_hover = version_str.clone();

    let active_op = operation_queue.active_operation_for(&version_str);
    let is_pending = operation_queue.has_pending_for_version(&version_str);
    let is_busy = active_op.is_some() || is_pending;

    let is_uninstalling = matches!(active_op, Some(Operation::Uninstall { .. }));
    let is_setting_default = matches!(active_op, Some(Operation::SetDefault { .. }));

    let is_hovered = hovered_version.as_ref().is_some_and(|h| h == &version_str);
    let show_actions = is_hovered || is_default;

    let mut row_content = row![text(version_display).size(14).width(Length::Fixed(120.0)),]
        .spacing(8)
        .align_y(Alignment::Center);

    if let Some(lts) = &version.lts_codename {
        row_content = row_content.push(
            container(text(format!("LTS: {}", lts)).size(11))
                .padding([2, 6])
                .style(styles::badge_lts),
        );
    }

    if is_default {
        row_content = row_content.push(
            container(text("default").size(11))
                .padding([2, 6])
                .style(styles::badge_default),
        );
    }

    row_content = row_content.push(Space::new().width(Length::Fill));

    if let Some(size) = version.disk_size {
        row_content = row_content.push(text(format_bytes(size)).size(12));
    }

    let action_style = if show_actions {
        styles::row_action_button
    } else {
        styles::row_action_button_hidden
    };
    let danger_style = if show_actions {
        styles::row_action_button_danger
    } else {
        styles::row_action_button_hidden
    };

    if show_actions {
        row_content = row_content.push(
            button(
                row![text("Changelog").size(11), icon::arrow_up_right(11.0),]
                    .spacing(2)
                    .align_y(Alignment::Center),
            )
            .on_press(Message::OpenChangelog(version_for_changelog))
            .style(action_style)
            .padding([4, 8]),
        );
    } else {
        row_content = row_content.push(
            button(text("Changelog").size(11))
                .style(action_style)
                .padding([4, 8]),
        );
    }

    if is_default {
        row_content = row_content.push(
            button(text("Default").size(12))
                .style(action_style)
                .padding([6, 12]),
        );
    } else if is_setting_default {
        row_content = row_content.push(
            button(text("Setting...").size(12))
                .style(action_style)
                .padding([6, 12]),
        );
    } else if is_busy || !show_actions {
        row_content = row_content.push(
            button(text("Set Default").size(12))
                .style(action_style)
                .padding([6, 12]),
        );
    } else {
        row_content = row_content.push(
            button(text("Set Default").size(12))
                .on_press(Message::SetDefault(version_for_default))
                .style(action_style)
                .padding([6, 12]),
        );
    }

    if is_uninstalling {
        row_content = row_content.push(
            button(text("Removing...").size(12))
                .style(danger_style)
                .padding([6, 12]),
        );
    } else if is_busy || !show_actions {
        row_content = row_content.push(
            button(text("Uninstall").size(12))
                .style(danger_style)
                .padding([6, 12]),
        );
    } else {
        row_content = row_content.push(
            button(text("Uninstall").size(12))
                .on_press(Message::RequestUninstall(version_str))
                .style(danger_style)
                .padding([6, 12]),
        );
    }

    let row_style = if is_hovered {
        styles::version_row_hovered
    } else {
        |_: &_| iced::widget::container::Style::default()
    };

    let row_container = container(row_content.padding([4, 8])).style(row_style);

    mouse_area(row_container)
        .on_enter(Message::VersionRowHovered(Some(version_for_hover)))
        .on_exit(Message::VersionRowHovered(None))
        .into()
}

pub(super) fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
