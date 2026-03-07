use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Alignment, Element, Length};

use versi_backend::{InstalledVersion, NodeVersion};

use crate::format::format_bytes;
use crate::icon;
use crate::message::Message;
use crate::state::{Operation, VersionSecurityFinding};
use crate::theme::styles;

use super::VersionListContext;

fn show_security_warning_icon(finding: Option<&VersionSecurityFinding>) -> bool {
    finding.is_some_and(VersionSecurityFinding::is_vulnerable)
}

pub(super) fn version_item_view<'a>(
    version: &'a InstalledVersion,
    default: Option<&'a versi_backend::NodeVersion>,
    ctx: &VersionListContext<'a>,
) -> Element<'a, Message> {
    let is_default = default.is_some_and(|d| d == &version.version);

    let version_str = version.version.to_string();
    let meta = ctx.metadata.and_then(|m| m.get(&version_str));
    let security_finding = ctx.security_findings.get(&version_str);

    let active_op = ctx.operation_queue.active_operation_for(version.version);
    let is_pending = ctx.operation_queue.has_pending_for_version(version.version);
    let is_busy = active_op.is_some() || is_pending;

    let is_uninstalling = matches!(active_op, Some(Operation::Uninstall { .. }));
    let is_setting_default = matches!(active_op, Some(Operation::SetDefault { .. }));

    let is_hovered = ctx
        .hovered_version
        .as_ref()
        .is_some_and(|h| h == &version_str);
    let show_actions = is_hovered || is_default;

    let mut row_content = row![
        container(text(version_str.clone()).size(14))
            .padding([2, 4])
            .width(Length::Fixed(crate::theme::tokens::COL_VERSION)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if show_security_warning_icon(security_finding) {
        row_content = row_content.push(container(icon::warning(13.0)).width(Length::Fixed(16.0)));
    }

    let row_content = push_badges_and_size(row_content, version, meta, is_default);

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

    let row_content = push_set_default_button(
        row_content,
        action_style,
        is_default,
        is_setting_default,
        is_busy || !show_actions,
        version.version,
    );
    let row_content = if ctx.supports_uninstall {
        push_uninstall_button(
            row_content,
            danger_style,
            is_uninstalling,
            is_busy || !show_actions,
            version.version,
        )
    } else {
        row_content
    };

    let row_style = if is_hovered {
        styles::version_row_hovered
    } else {
        |_: &_| iced::widget::container::Style::default()
    };

    let row_container = container(row_content.padding([4, 8]))
        .style(row_style)
        .width(Length::Fill);

    mouse_area(row_container)
        .on_press(Message::ShowVersionDetail(version_str.clone()))
        .on_enter(Message::VersionRowHovered(Some(version_str.clone())))
        .on_exit(Message::VersionRowHovered(None))
        .on_right_press(Message::ShowContextMenu {
            version: version_str,
            is_installed: true,
            is_default,
        })
        .into()
}

fn push_badges_and_size<'a>(
    mut row_content: iced::widget::Row<'a, Message>,
    version: &'a InstalledVersion,
    meta: Option<&'a versi_core::VersionMeta>,
    is_default: bool,
) -> iced::widget::Row<'a, Message> {
    if let Some(lts) = &version.lts_codename {
        row_content = row_content.push(
            container(text(format!("LTS: {lts}")).size(11))
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

    if meta.is_some_and(|version_meta| version_meta.security) {
        row_content = row_content.push(
            container(text("Security").size(11))
                .padding([2, 6])
                .style(styles::badge_security),
        );
    }

    row_content = row_content.push(Space::new().width(Length::Fill));
    if let Some(size) = version.disk_size {
        row_content = row_content.push(text(format_bytes(size)).size(12));
    }
    row_content
}

fn push_set_default_button(
    row_content: iced::widget::Row<'_, Message>,
    action_style: fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style,
    is_default: bool,
    is_setting_default: bool,
    is_disabled: bool,
    version: NodeVersion,
) -> iced::widget::Row<'_, Message> {
    let button = if is_default {
        button(text("Default").size(12))
    } else if is_setting_default {
        button(text("Setting...").size(12))
    } else {
        button(text("Set Default").size(12))
    };

    if !is_default && !is_setting_default && !is_disabled {
        row_content.push(
            button
                .on_press(Message::SetDefault(version))
                .style(action_style)
                .padding([6, 12]),
        )
    } else {
        row_content.push(button.style(action_style).padding([6, 12]))
    }
}

fn push_uninstall_button(
    row_content: iced::widget::Row<'_, Message>,
    danger_style: fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style,
    is_uninstalling: bool,
    is_disabled: bool,
    version: NodeVersion,
) -> iced::widget::Row<'_, Message> {
    let button = if is_uninstalling {
        button(text("Removing...").size(12))
    } else {
        button(text("Uninstall").size(12))
    };

    if !is_uninstalling && !is_disabled {
        row_content.push(
            button
                .on_press(Message::RequestUninstall(version))
                .style(danger_style)
                .padding([6, 12]),
        )
    } else {
        row_content.push(button.style(danger_style).padding([6, 12]))
    }
}

#[cfg(test)]
mod tests {
    use super::show_security_warning_icon;
    use crate::state::VersionSecurityFinding;

    #[test]
    fn show_security_warning_icon_requires_vulnerable_finding() {
        assert!(!show_security_warning_icon(None));
        assert!(!show_security_warning_icon(Some(&VersionSecurityFinding {
            advisory_ids: Vec::new(),
            is_eol: false,
        })));
        assert!(show_security_warning_icon(Some(&VersionSecurityFinding {
            advisory_ids: vec!["163".to_string()],
            is_eol: false,
        })));
        assert!(show_security_warning_icon(Some(&VersionSecurityFinding {
            advisory_ids: Vec::new(),
            is_eol: true,
        })));
    }
}
