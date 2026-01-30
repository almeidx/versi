use iced::widget::{Space, button, column, container, mouse_area, row, text, text_input, tooltip};
use iced::{Alignment, Element, Length};

use crate::icon;
use crate::message::Message;
use crate::settings::AppSettings;
use crate::state::{MainState, Modal, NetworkStatus};
use crate::theme::styles;
use crate::widgets::{toast_container, version_list};

pub fn view<'a>(state: &'a MainState, settings: &'a AppSettings) -> Element<'a, Message> {
    let header = header_view(state);
    let search_bar = search_bar_view(state);
    let hovered = if state.modal.is_some() {
        &None
    } else {
        &state.hovered_version
    };
    let version_list = version_list::view(
        state.active_environment(),
        &state.search_query,
        &state.available_versions.versions,
        state.available_versions.schedule.as_ref(),
        &state.operation_queue,
        hovered,
    );

    let mut main_column = column![].spacing(0);

    let has_tabs = if let Some(tabs) = environment_tabs_view(state) {
        main_column = main_column.push(
            container(tabs).padding(iced::Padding::new(0.0).top(16.0).left(32.0).right(32.0)),
        );
        true
    } else {
        false
    };

    let right_inset = iced::Padding::new(0.0).right(32.0);
    let mut content_column = column![
        container(header).padding(right_inset),
        container(search_bar).padding(right_inset),
    ]
    .spacing(20);

    if let Some(banners) = contextual_banners(state) {
        content_column = content_column.push(container(banners).padding(right_inset));
    }

    content_column = content_column.push(version_list);

    let content_padding = if has_tabs {
        iced::Padding::new(32.0).right(0.0)
    } else {
        iced::Padding::new(32.0).top(16.0).right(0.0)
    };
    let main_content = content_column.padding(content_padding);

    main_column = main_column.push(main_content);

    let with_modal: Element<Message> = if let Some(modal) = &state.modal {
        modal_overlay(main_column.into(), modal, state, settings)
    } else {
        main_column.into()
    };

    toast_container::view(with_modal, &state.toasts)
}

fn header_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    let env = state.active_environment();

    let subtitle = match &env.fnm_version {
        Some(fnm_v) => format!("fnm {}", fnm_v),
        None => String::new(),
    };

    let title_section =
        column![text("Node Versions").size(32), text(subtitle).size(13),].spacing(4);

    let mut icon_row = row![].spacing(4).align_y(Alignment::Center);

    if let Some(update) = &state.app_update {
        icon_row = icon_row.push(
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

    if let Some(update) = &state.fnm_update {
        icon_row = icon_row.push(
            button(
                container(
                    row![
                        text(format!("fnm {} available", update.latest_version)).size(11),
                        icon::arrow_up_right(11.0),
                    ]
                    .spacing(2)
                    .align_y(Alignment::Center),
                )
                .padding([2, 8]),
            )
            .on_press(Message::OpenFnmUpdate)
            .style(styles::app_update_button)
            .padding(0),
        );
    }

    icon_row = icon_row.push(tooltip(
        button(icon::refresh(16.0))
            .on_press(Message::RefreshEnvironment)
            .style(styles::ghost_button)
            .padding([6, 8]),
        text("Refresh").size(12),
        tooltip::Position::Bottom,
    ));

    icon_row = icon_row.push(tooltip(
        button(icon::settings(16.0))
            .on_press(Message::NavigateToSettings)
            .style(styles::ghost_button)
            .padding([6, 8]),
        text("Settings").size(12),
        tooltip::Position::Bottom,
    ));

    icon_row = icon_row.push(tooltip(
        button(icon::info(16.0))
            .on_press(Message::NavigateToAbout)
            .style(styles::ghost_button)
            .padding([6, 8]),
        text("About").size(12),
        tooltip::Position::Bottom,
    ));

    row![title_section, Space::new().width(Length::Fill), icon_row,]
        .align_y(Alignment::Center)
        .into()
}

fn search_bar_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    let input = text_input(
        "Search or install versions (e.g., '22', 'lts')...",
        &state.search_query,
    )
    .on_input(Message::SearchChanged)
    .padding(14)
    .size(14)
    .style(styles::search_input);

    let clear_btn: Element<Message> = if state.search_query.is_empty() {
        Space::new().into()
    } else {
        tooltip(
            button(icon::close(14.0))
                .on_press(Message::SearchChanged(String::new()))
                .style(styles::ghost_button)
                .padding([6, 10]),
            text("Clear search").size(12),
            tooltip::Position::Left,
        )
        .into()
    };

    iced::widget::stack![
        input,
        container(clear_btn)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding::new(0.0).right(4.0)),
    ]
    .into()
}

fn contextual_banners<'a>(state: &'a MainState) -> Option<Element<'a, Message>> {
    let env = state.active_environment();
    let schedule = state.available_versions.schedule.as_ref();
    let remote = &state.available_versions.versions;

    let mut banners: Vec<Element<Message>> = Vec::new();

    match state.available_versions.network_status() {
        NetworkStatus::Offline(_) => {
            banners.push(
                button(
                    row![
                        text("Could not load available versions").size(13),
                        Space::new().width(Length::Fill),
                        text("Retry").size(13),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::FetchRemoteVersions)
                .style(styles::banner_button_warning)
                .padding([12, 16])
                .width(Length::Fill)
                .into(),
            );
        }
        NetworkStatus::Stale(_) => {
            banners.push(
                button(
                    row![
                        text("Using cached data \u{2014} could not refresh from network").size(13),
                        Space::new().width(Length::Fill),
                        text("Retry").size(13),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::FetchRemoteVersions)
                .style(styles::banner_button_warning)
                .padding([12, 16])
                .width(Length::Fill)
                .into(),
            );
        }
        _ => {}
    }

    if state.available_versions.schedule_error.is_some() && schedule.is_none() {
        banners.push(
            button(
                row![
                    text("Release schedule unavailable \u{2014} EOL detection may be inaccurate")
                        .size(13),
                    Space::new().width(Length::Fill),
                    text("Retry").size(13),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::FetchReleaseSchedule)
            .style(styles::banner_button_warning)
            .padding([12, 16])
            .width(Length::Fill)
            .into(),
        );
    }

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

fn modal_overlay<'a>(
    content: Element<'a, Message>,
    modal: &'a Modal,
    _state: &'a MainState,
    _settings: &'a AppSettings,
) -> Element<'a, Message> {
    let modal_content: Element<Message> = match modal {
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
