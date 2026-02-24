use std::collections::HashMap;

use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Element, Length};

use versi_core::VersionMeta;

use crate::message::Message;
use crate::settings::AppSettings;
use crate::state::{MainState, Modal};
use crate::theme::styles;

fn version_preview_list(labels: Vec<String>, preview_limit: usize) -> Element<'static, Message> {
    let muted = crate::theme::tokens::TEXT_MUTED;
    let total = labels.len();
    let mut list = column![].spacing(4);
    for label in labels.into_iter().take(preview_limit) {
        list = list.push(text(label).size(12).color(muted));
    }
    if total > preview_limit {
        list = list.push(
            text(format!("...and {} more", total - preview_limit))
                .size(11)
                .color(muted),
        );
    }
    list.into()
}

pub(super) fn modal_overlay<'a>(
    content: Element<'a, Message>,
    modal: &'a Modal,
    state: &'a MainState,
    settings: &'a AppSettings,
) -> Element<'a, Message> {
    let preview_limit = settings.modal_preview_limit;
    let modal_content: Element<Message> = match modal {
        Modal::ConfirmBulkUpdateMajors { versions } => {
            confirm_bulk_update_view(versions, preview_limit)
        }
        Modal::ConfirmBulkUninstallEOL { versions } => {
            confirm_bulk_uninstall_eol_view(versions, preview_limit)
        }
        Modal::ConfirmBulkUninstallMajor { major, versions } => {
            confirm_bulk_uninstall_major_view(*major, versions, preview_limit)
        }
        Modal::ConfirmBulkUninstallMajorExceptLatest {
            major,
            versions,
            keeping,
        } => confirm_bulk_uninstall_major_except_latest_view(
            *major,
            versions,
            keeping,
            preview_limit,
        ),
        Modal::ConfirmUninstallDefault { version } => confirm_uninstall_default_view(version),
        Modal::KeyboardShortcuts => keyboard_shortcuts_view(),
        Modal::VersionDetail { version } => {
            version_detail_view(version, state.available_versions.metadata.as_ref(), state)
        }
    };

    let backdrop = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .style(|_theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.4,
                })),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::CloseModal);

    let modal_container = mouse_area(
        container(modal_content)
            .style(styles::modal_container)
            .padding(iced::Padding::new(crate::theme::tokens::MODAL_PADDING))
            .max_width(crate::theme::tokens::MODAL_MAX_WIDTH),
    )
    .on_press(Message::NoOp);

    let modal_layer = container(modal_container)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill);

    iced::widget::stack![content, backdrop, modal_layer].into()
}

fn confirm_bulk_update_view(
    versions: &[(String, String)],
    preview_limit: usize,
) -> Element<'_, Message> {
    let labels: Vec<String> = versions
        .iter()
        .map(|(from, to)| format!("{from} \u{2192} {to}"))
        .collect();

    column![
        text("Update All Versions?").size(20),
        Space::new().height(12),
        text(format!(
            "This will install {} newer version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_preview_list(labels, preview_limit),
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelBulkOperation)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Update All").size(13))
                .on_press(Message::ConfirmBulkUpdateMajors)
                .style(styles::primary_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn confirm_bulk_uninstall_eol_view(
    versions: &[String],
    preview_limit: usize,
) -> Element<'_, Message> {
    let labels: Vec<String> = versions.iter().map(|v| format!("Node {v}")).collect();

    column![
        text("Remove All EOL Versions?").size(20),
        Space::new().height(12),
        text(format!(
            "This will uninstall {} end-of-life version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_preview_list(labels, preview_limit),
        Space::new().height(8),
        text("These versions no longer receive security updates.")
            .size(12)
            .color(crate::theme::tokens::EOL_ORANGE),
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelBulkOperation)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Remove All").size(13))
                .on_press(Message::ConfirmBulkUninstallEOL)
                .style(styles::danger_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn confirm_bulk_uninstall_major_view(
    major: u32,
    versions: &[String],
    preview_limit: usize,
) -> Element<'_, Message> {
    let labels: Vec<String> = versions.iter().map(|v| format!("Node {v}")).collect();

    column![
        text(format!("Remove All Node {major}.x Versions?")).size(20),
        Space::new().height(12),
        text(format!(
            "This will uninstall {} version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_preview_list(labels, preview_limit),
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelBulkOperation)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Remove All").size(13))
                .on_press(Message::ConfirmBulkUninstallMajor { major })
                .style(styles::danger_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn confirm_bulk_uninstall_major_except_latest_view<'a>(
    major: u32,
    versions: &'a [String],
    keeping: &'a str,
    preview_limit: usize,
) -> Element<'a, Message> {
    let labels: Vec<String> = versions.iter().map(|v| format!("Node {v}")).collect();

    column![
        text(format!("Clean Up Node {major}.x Versions?")).size(20),
        Space::new().height(12),
        text(format!(
            "This will uninstall {} older version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_preview_list(labels, preview_limit),
        Space::new().height(8),
        text(format!("Node {keeping} will be kept."))
            .size(12)
            .color(iced::Color::from_rgb8(52, 199, 89)),
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelBulkOperation)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Remove Older").size(13))
                .on_press(Message::ConfirmBulkUninstallMajorExceptLatest { major })
                .style(styles::danger_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn confirm_uninstall_default_view(version: &str) -> Element<'_, Message> {
    column![
        text("Uninstall Default Version?").size(20),
        Space::new().height(12),
        text(format!(
            "Node {version} is your current default version. Uninstalling it will leave no default set."
        ))
        .size(14),
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CloseModal)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Uninstall").size(13))
                .on_press(Message::ConfirmUninstallDefault(version.to_string()))
                .style(styles::danger_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn version_detail_view<'a>(
    version: &'a str,
    metadata: Option<&'a HashMap<String, VersionMeta>>,
    state: &'a MainState,
) -> Element<'a, Message> {
    let muted = crate::theme::tokens::TEXT_MUTED;
    let meta = metadata.and_then(|m| m.get(version));

    let mut content = column![text(format!("Node {version}")).size(20),].spacing(4);

    content = content.push(Space::new().height(12));

    if let Some(meta) = meta {
        content = content.push(
            text(format!("Released {}", meta.date))
                .size(13)
                .color(muted),
        );
        content = content.push(Space::new().height(8));

        let mut badge_row = row![].spacing(8).align_y(iced::Alignment::Center);

        if meta.security {
            badge_row = badge_row.push(
                container(text("Security Release").size(11))
                    .padding([2, 6])
                    .style(styles::badge_security),
            );
        }

        if let Some(lts) = lookup_lts(version, state) {
            badge_row = badge_row.push(
                container(text(format!("LTS: {lts}")).size(11))
                    .padding([2, 6])
                    .style(styles::badge_lts),
            );
        }

        content = content.push(badge_row);
        content = content.push(Space::new().height(12));

        let mut details = column![].spacing(6);
        if let Some(npm) = &meta.npm {
            details = details.push(meta_row("npm", npm, muted));
        }
        if let Some(v8) = &meta.v8 {
            details = details.push(meta_row("V8", v8, muted));
        }
        if let Some(openssl) = &meta.openssl {
            details = details.push(meta_row("OpenSSL", openssl, muted));
        }
        content = content.push(details);

        if is_major_release(version) {
            content = content.push(Space::new().height(8));
            content = content.push(
                text("Major release — check changelog for breaking changes")
                    .size(12)
                    .color(crate::theme::tokens::EOL_ORANGE),
            );
        }
    } else {
        content = content.push(text("No metadata available").size(13).color(muted));
    }

    content = content.push(Space::new().height(24));
    content = content.push(
        row![
            button(text("Close").size(13))
                .on_press(Message::CloseModal)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("View Full Changelog").size(13))
                .on_press(Message::OpenChangelog(version.to_string()))
                .style(styles::primary_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    );

    content.width(Length::Fill).into()
}

fn meta_row<'a>(label: &'a str, value: &'a str, muted: iced::Color) -> Element<'a, Message> {
    row![
        text(label)
            .size(12)
            .width(Length::Fixed(crate::theme::tokens::COL_META_LABEL))
            .color(muted),
        text(value).size(12),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn lookup_lts<'a>(version: &str, state: &'a MainState) -> Option<&'a str> {
    let parsed = version.parse::<versi_backend::NodeVersion>().ok()?;
    state
        .available_versions
        .versions
        .iter()
        .find(|v| v.version == parsed)
        .and_then(|v| v.lts_codename.as_deref())
}

fn is_major_release(version: &str) -> bool {
    let trimmed = version.strip_prefix('v').unwrap_or(version);
    trimmed.ends_with(".0.0")
}

fn keyboard_shortcuts_view() -> Element<'static, Message> {
    #[cfg(target_os = "macos")]
    let mod_key = "\u{2318}";
    #[cfg(not(target_os = "macos"))]
    let mod_key = "Ctrl+";

    let shortcuts = [
        (format!("{mod_key}K"), "Search versions"),
        (format!("{mod_key}R"), "Refresh"),
        (format!("{mod_key},"), "Settings"),
        (format!("{mod_key}W"), "Close window"),
        (format!("{mod_key}Tab"), "Next environment"),
        (format!("{mod_key}Shift+Tab"), "Previous environment"),
        ("\u{2191}/\u{2193}".to_string(), "Navigate versions"),
        ("Tab/Shift+Tab".to_string(), "Navigate versions"),
        ("I".to_string(), "Install hovered"),
        ("D".to_string(), "Set hovered as default"),
        ("U".to_string(), "Uninstall hovered"),
        ("Enter".to_string(), "Install / set default"),
        ("Esc".to_string(), "Close modal"),
        ("?".to_string(), "This help"),
    ];

    let muted = crate::theme::tokens::TEXT_MUTED;

    let mut rows = column![].spacing(8);
    for (key, desc) in shortcuts {
        rows = rows.push(
            row![
                container(text(key).size(12))
                    .style(styles::kbd_container)
                    .padding([2, 8])
                    .width(Length::Fixed(crate::theme::tokens::COL_KBD_KEY)),
                text(desc).size(13).color(muted),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        );
    }

    column![
        text("Keyboard Shortcuts").size(20),
        Space::new().height(16),
        rows,
        Space::new().height(24),
        button(text("Close").size(13))
            .on_press(Message::CloseModal)
            .style(styles::secondary_button)
            .padding([10, 20]),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}
