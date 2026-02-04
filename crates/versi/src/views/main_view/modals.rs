use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Element, Length};

use crate::message::Message;
use crate::settings::AppSettings;
use crate::state::{MainState, Modal};
use crate::theme::styles;

pub(super) fn modal_overlay<'a>(
    content: Element<'a, Message>,
    modal: &'a Modal,
    _state: &'a MainState,
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
        Modal::KeyboardShortcuts => keyboard_shortcuts_view(),
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
            .padding(28)
            .max_width(480),
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
    let mut version_list = column![].spacing(4);

    for (from, to) in versions.iter().take(preview_limit) {
        version_list = version_list.push(
            text(format!("{} → {}", from, to))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > preview_limit {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - preview_limit))
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    column![
        text("Update All Versions?").size(20),
        Space::new().height(12),
        text(format!(
            "This will install {} newer version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_list,
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
    let mut version_list = column![].spacing(4);

    for version in versions.iter().take(preview_limit) {
        version_list = version_list.push(
            text(format!("Node {}", version))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > preview_limit {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - preview_limit))
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    column![
        text("Remove All EOL Versions?").size(20),
        Space::new().height(12),
        text(format!(
            "This will uninstall {} end-of-life version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_list,
        Space::new().height(8),
        text("These versions no longer receive security updates.")
            .size(12)
            .color(iced::Color::from_rgb8(255, 149, 0)),
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
    let mut version_list = column![].spacing(4);

    for version in versions.iter().take(preview_limit) {
        version_list = version_list.push(
            text(format!("Node {}", version))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > preview_limit {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - preview_limit))
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    column![
        text(format!("Remove All Node {}.x Versions?", major)).size(20),
        Space::new().height(12),
        text(format!(
            "This will uninstall {} version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_list,
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
    let mut version_list = column![].spacing(4);

    for version in versions.iter().take(preview_limit) {
        version_list = version_list.push(
            text(format!("Node {}", version))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > preview_limit {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - preview_limit))
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    column![
        text(format!("Clean Up Node {}.x Versions?", major)).size(20),
        Space::new().height(12),
        text(format!(
            "This will uninstall {} older version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_list,
        Space::new().height(8),
        text(format!("Node {} will be kept.", keeping))
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

fn keyboard_shortcuts_view() -> Element<'static, Message> {
    #[cfg(target_os = "macos")]
    let mod_key = "\u{2318}";
    #[cfg(not(target_os = "macos"))]
    let mod_key = "Ctrl+";

    let shortcuts = [
        (format!("{}K", mod_key), "Search versions"),
        (format!("{}R", mod_key), "Refresh"),
        (format!("{},", mod_key), "Settings"),
        (format!("{}W", mod_key), "Close window"),
        (format!("{}Tab", mod_key), "Next environment"),
        (format!("{}Shift+Tab", mod_key), "Previous environment"),
        ("\u{2191}/\u{2193}".to_string(), "Navigate versions"),
        ("Enter".to_string(), "Install / set default"),
        ("Esc".to_string(), "Close modal"),
        ("?".to_string(), "This help"),
    ];

    let muted = iced::Color::from_rgb8(142, 142, 147);

    let mut rows = column![].spacing(8);
    for (key, desc) in shortcuts {
        rows = rows.push(
            row![
                container(text(key).size(12))
                    .style(styles::kbd_container)
                    .padding([2, 8])
                    .width(Length::Fixed(80.0)),
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
