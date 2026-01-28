use iced::widget::{Space, button, column, container, mouse_area, row, text, text_input};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::settings::AppSettings;
use crate::state::{MainState, Modal, Operation, QueuedOperation};
use crate::theme::styles;
use crate::widgets::{toast_container, version_list};

pub fn view<'a>(state: &'a MainState, settings: &'a AppSettings) -> Element<'a, Message> {
    let header = header_view(state);
    let search_bar = search_bar_view(state);
    let version_list = version_list::view(
        state.active_environment(),
        &state.search_query,
        &state.available_versions.versions,
        state.available_versions.schedule.as_ref(),
        &state.operation_queue,
        &state.hovered_version,
    );

    let mut main_column = column![].spacing(0);

    if let Some(tabs) = environment_tabs_view(state) {
        main_column = main_column.push(
            container(tabs).padding(iced::Padding::new(0.0).top(16.0).left(32.0).right(32.0)),
        );
    }

    let mut content_column = column![header, search_bar].spacing(20);

    if let Some(banners) = contextual_banners(state) {
        content_column = content_column.push(banners);
    }

    content_column = content_column.push(version_list);

    let main_content = content_column.padding(32);

    main_column = main_column.push(main_content);

    let with_modal: Element<Message> = if let Some(modal) = &state.modal {
        modal_overlay(main_column.into(), modal, state, settings)
    } else {
        main_column.into()
    };

    let with_toasts = toast_container::view(with_modal, &state.toasts);

    if let Some(operation) = operation_status_view(state) {
        let operation_overlay = container(operation)
            .padding(16)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Bottom)
            .width(Length::Fill)
            .height(Length::Fill);

        iced::widget::stack![with_toasts, operation_overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        with_toasts
    }
}

fn header_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    let env = state.active_environment();

    let subtitle = match (&env.default_version, &env.fnm_version) {
        (Some(v), Some(fnm_v)) => format!("Default: {} \u{00b7} fnm {}", v, fnm_v),
        (Some(v), None) => format!("Default: {}", v),
        (None, Some(fnm_v)) => format!("fnm {}", fnm_v),
        (None, None) => "No default set".to_string(),
    };

    let title_section =
        column![text("Node Versions").size(32), text(subtitle).size(13),].spacing(4);

    let mut icon_row = row![].spacing(4).align_y(Alignment::Center);

    if let Some(update) = &state.app_update {
        icon_row = icon_row.push(
            button(
                container(text(format!("v{} available", update.latest_version)).size(11))
                    .padding([2, 8]),
            )
            .on_press(Message::OpenAppUpdate)
            .style(styles::app_update_button)
            .padding(0),
        );
    }

    if let Some(update) = &state.fnm_update {
        icon_row = icon_row.push(
            button(
                container(text(format!("fnm {} available", update.latest_version)).size(11))
                    .padding([2, 8]),
            )
            .on_press(Message::OpenFnmUpdate)
            .style(styles::app_update_button)
            .padding(0),
        );
    }

    icon_row = icon_row.push(
        button(text("\u{21bb}").size(16))
            .on_press(Message::RefreshEnvironment)
            .style(styles::ghost_button)
            .padding([6, 8]),
    );

    icon_row = icon_row.push(
        button(text("\u{2699}").size(16))
            .on_press(Message::NavigateToSettings)
            .style(styles::ghost_button)
            .padding([6, 8]),
    );

    row![title_section, Space::new().width(Length::Fill), icon_row,]
        .align_y(Alignment::Center)
        .into()
}

fn search_bar_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    text_input(
        "Search or install versions (e.g., '22', 'lts')...",
        &state.search_query,
    )
    .on_input(Message::SearchChanged)
    .padding(14)
    .size(14)
    .style(styles::search_input)
    .into()
}

fn contextual_banners<'a>(state: &'a MainState) -> Option<Element<'a, Message>> {
    let env = state.active_environment();
    let schedule = state.available_versions.schedule.as_ref();
    let remote = &state.available_versions.versions;

    let mut banners: Vec<Element<Message>> = Vec::new();

    let update_count = {
        let mut latest_by_major: std::collections::HashMap<u32, &versi_core::NodeVersion> =
            std::collections::HashMap::new();
        for v in remote {
            latest_by_major
                .entry(v.version.major)
                .and_modify(|existing| {
                    if &v.version > *existing {
                        *existing = &v.version;
                    }
                })
                .or_insert(&v.version);
        }

        env.version_groups
            .iter()
            .filter(|group| {
                let installed_latest = group.versions.iter().map(|v| &v.version).max();
                latest_by_major.get(&group.major).is_some_and(|latest| {
                    installed_latest.is_some_and(|installed| *latest > installed)
                })
            })
            .count()
    };

    if update_count > 0 {
        banners.push(
            button(
                row![
                    text(format!(
                        "{} major {} with updates available",
                        update_count,
                        if update_count == 1 {
                            "version"
                        } else {
                            "versions"
                        }
                    ))
                    .size(13),
                    Space::new().width(Length::Fill),
                    text("Update All").size(13),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::RequestBulkUpdateMajors)
            .style(styles::banner_button_info)
            .padding([12, 16])
            .width(Length::Fill)
            .into(),
        );
    }

    let eol_count = schedule
        .map(|s| {
            env.version_groups
                .iter()
                .filter(|g| !s.is_active(g.major))
                .map(|g| g.versions.len())
                .sum::<usize>()
        })
        .unwrap_or(0);

    if eol_count > 0 {
        banners.push(
            button(
                row![
                    text(format!(
                        "{} end-of-life {} installed",
                        eol_count,
                        if eol_count == 1 {
                            "version"
                        } else {
                            "versions"
                        }
                    ))
                    .size(13),
                    Space::new().width(Length::Fill),
                    text("Clean Up").size(13),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::RequestBulkUninstallEOL)
            .style(styles::banner_button_warning)
            .padding([12, 16])
            .width(Length::Fill)
            .into(),
        );
    }

    if banners.is_empty() {
        None
    } else {
        Some(column(banners).spacing(8).into())
    }
}

fn environment_tabs_view<'a>(state: &'a MainState) -> Option<Element<'a, Message>> {
    if state.environments.len() <= 1 {
        return None;
    }

    let tabs: Vec<_> = state
        .environments
        .iter()
        .enumerate()
        .map(|(idx, env)| {
            let is_active = idx == state.active_environment_idx;

            if !env.available {
                let label = if let Some(reason) = &env.error {
                    format!("{} ({})", env.name, reason)
                } else {
                    format!("{} (Unavailable)", env.name)
                };
                return button(text(label).size(13))
                    .style(styles::disabled_tab_button)
                    .padding([8, 16])
                    .into();
            }

            let style = if is_active {
                styles::active_tab_button
            } else {
                styles::inactive_tab_button
            };

            button(text(&env.name).size(13))
                .on_press(Message::EnvironmentSelected(idx))
                .style(style)
                .padding([8, 16])
                .into()
        })
        .collect();

    Some(row(tabs).spacing(4).into())
}

fn operation_status_view<'a>(state: &'a MainState) -> Option<Element<'a, Message>> {
    let queue = &state.operation_queue;

    if queue.active_installs.is_empty() && queue.exclusive_op.is_none() && queue.pending.is_empty()
    {
        return None;
    }

    let mut content = column![].spacing(8);

    for op in &queue.active_installs {
        content = content.push(current_operation_view(op));
    }

    if let Some(op) = &queue.exclusive_op {
        content = content.push(current_operation_view(op));
    }

    if !queue.pending.is_empty() {
        content = content.push(text("Queued").size(11));
        for queued in &queue.pending {
            content = content.push(queued_operation_view(queued));
        }
    }

    Some(
        container(content.padding(16))
            .style(styles::card_container)
            .max_width(320)
            .into(),
    )
}

fn current_operation_view(op: &Operation) -> Element<'_, Message> {
    match op {
        Operation::Install { version, progress } => {
            let phase_text = match progress.phase {
                versi_core::InstallPhase::Starting => "Preparing...",
                versi_core::InstallPhase::Downloading => "Downloading...",
                versi_core::InstallPhase::Extracting => "Extracting...",
                versi_core::InstallPhase::Installing => "Installing...",
                versi_core::InstallPhase::Complete => "Complete!",
                versi_core::InstallPhase::Failed => "Failed",
            };

            column![
                text(format!("Installing Node {}", version)).size(14),
                text(phase_text).size(12),
            ]
            .spacing(4)
            .into()
        }
        Operation::Uninstall { version } => text(format!("Removing Node {}...", version))
            .size(14)
            .into(),
        Operation::SetDefault { version } => text(format!("Setting default to {}...", version))
            .size(14)
            .into(),
    }
}

fn queued_operation_view(queued: &QueuedOperation) -> Element<'_, Message> {
    row![
        text(queued.request.description()).size(12),
        Space::new().width(Length::Fill),
        button(text("×").size(12))
            .on_press(Message::CancelQueuedOperation(queued.id))
            .style(styles::ghost_button)
            .padding([2, 6]),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn modal_overlay<'a>(
    content: Element<'a, Message>,
    modal: &'a Modal,
    _state: &'a MainState,
    _settings: &'a AppSettings,
) -> Element<'a, Message> {
    let modal_content: Element<Message> = match modal {
        Modal::ConfirmUninstall {
            version,
            is_default,
        } => confirm_uninstall_view(version, *is_default),
        Modal::ConfirmBulkUpdateMajors { versions } => confirm_bulk_update_view(versions),
        Modal::ConfirmBulkUninstallEOL { versions } => confirm_bulk_uninstall_eol_view(versions),
        Modal::ConfirmBulkUninstallMajor { major, versions } => {
            confirm_bulk_uninstall_major_view(*major, versions)
        }
        Modal::ConfirmBulkUninstallMajorExceptLatest {
            major,
            versions,
            keeping,
        } => confirm_bulk_uninstall_major_except_latest_view(*major, versions, keeping),
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
            .style(styles::card_container)
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

fn confirm_uninstall_view<'a>(version: &'a str, is_default: bool) -> Element<'a, Message> {
    let mut content = column![
        text(format!("Remove Node {}?", version)).size(20),
        Space::new().height(12),
    ]
    .spacing(4);

    if is_default {
        content = content
            .push(
                text("Warning: This is your default Node.js version.")
                    .size(14)
                    .color(styles::WARNING_COLOR),
            )
            .push(Space::new().height(4))
            .push(
                text("You will need to set a new default after uninstalling.")
                    .size(14)
                    .color(styles::WARNING_COLOR),
            );
    } else {
        content = content.push(text("This version will be uninstalled from your system.").size(14));
    }

    content = content.push(Space::new().height(24)).push(
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelUninstall)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Remove").size(13))
                .on_press(Message::ConfirmUninstall(version.to_string()))
                .style(styles::danger_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    );

    content.width(Length::Fill).into()
}

fn confirm_bulk_update_view(versions: &[(String, String)]) -> Element<'_, Message> {
    let mut version_list = column![].spacing(4);

    for (from, to) in versions.iter().take(10) {
        version_list = version_list.push(
            text(format!("{} → {}", from, to))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > 10 {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - 10))
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

fn confirm_bulk_uninstall_eol_view(versions: &[String]) -> Element<'_, Message> {
    let mut version_list = column![].spacing(4);

    for version in versions.iter().take(10) {
        version_list = version_list.push(
            text(format!("Node {}", version))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > 10 {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - 10))
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

fn confirm_bulk_uninstall_major_view(major: u32, versions: &[String]) -> Element<'_, Message> {
    let mut version_list = column![].spacing(4);

    for version in versions.iter().take(10) {
        version_list = version_list.push(
            text(format!("Node {}", version))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > 10 {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - 10))
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
) -> Element<'a, Message> {
    let mut version_list = column![].spacing(4);

    for version in versions.iter().take(10) {
        version_list = version_list.push(
            text(format!("Node {}", version))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > 10 {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - 10))
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
