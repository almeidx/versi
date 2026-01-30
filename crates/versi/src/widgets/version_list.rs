use std::collections::{HashMap, HashSet};

use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Element, Length};

use versi_core::{InstalledVersion, NodeVersion, ReleaseSchedule, RemoteVersion, VersionGroup};

use crate::icon;
use crate::message::Message;
use crate::state::{EnvironmentState, Operation, OperationQueue};
use crate::theme::styles;

fn compute_latest_by_major(remote_versions: &[RemoteVersion]) -> HashMap<u32, NodeVersion> {
    let mut latest: HashMap<u32, NodeVersion> = HashMap::new();

    for v in remote_versions {
        let major = v.version.major;
        latest
            .entry(major)
            .and_modify(|existing| {
                if v.version > *existing {
                    *existing = v.version.clone();
                }
            })
            .or_insert_with(|| v.version.clone());
    }

    latest
}

fn filter_available_versions<'a>(
    versions: &'a [RemoteVersion],
    query: &str,
) -> Vec<&'a RemoteVersion> {
    let query_lower = query.to_lowercase();

    let mut filtered: Vec<&RemoteVersion> = versions
        .iter()
        .filter(|v| {
            let version_str = v.version.to_string();

            if query_lower == "lts" {
                return v.lts_codename.is_some();
            }

            version_str.contains(query)
                || v.lts_codename
                    .as_ref()
                    .map(|c| c.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
        })
        .collect();

    filtered.sort_by(|a, b| b.version.cmp(&a.version));

    let mut latest_by_minor: HashMap<(u32, u32), &RemoteVersion> = HashMap::new();
    for v in &filtered {
        let key = (v.version.major, v.version.minor);
        latest_by_minor
            .entry(key)
            .and_modify(|existing| {
                if v.version.patch > existing.version.patch {
                    *existing = v;
                }
            })
            .or_insert(v);
    }

    let mut result: Vec<&RemoteVersion> = latest_by_minor.into_values().collect();
    result.sort_by(|a, b| b.version.cmp(&a.version));
    result.truncate(20);
    result
}

pub fn view<'a>(
    env: &'a EnvironmentState,
    search_query: &'a str,
    remote_versions: &'a [RemoteVersion],
    schedule: Option<&'a ReleaseSchedule>,
    operation_queue: &'a OperationQueue,
    hovered_version: &'a Option<String>,
) -> Element<'a, Message> {
    let latest_by_major = compute_latest_by_major(remote_versions);

    if env.loading && env.installed_versions.is_empty() {
        return container(
            column![text("Loading versions...").size(16),]
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    if let Some(error) = &env.error {
        return container(
            column![
                text("Error loading versions").size(16),
                text(error).size(14),
                Space::new().height(16),
                button(text("Retry"))
                    .on_press(Message::RefreshEnvironment)
                    .style(styles::primary_button)
                    .padding([8, 16]),
            ]
            .spacing(8)
            .align_x(Alignment::Center),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let installed_set: HashSet<String> = env
        .installed_versions
        .iter()
        .map(|v| v.version.to_string())
        .collect();

    let filtered_groups: Vec<&VersionGroup> = env
        .version_groups
        .iter()
        .filter(|group| filter_group(group, search_query))
        .collect();

    let default_version = &env.default_version;

    let mut content_items: Vec<Element<Message>> = Vec::new();

    if !filtered_groups.is_empty() && search_query.is_empty() {
        for group in &filtered_groups {
            let installed_latest = group.versions.iter().map(|v| &v.version).max();
            let update_available = latest_by_major.get(&group.major).and_then(|latest| {
                installed_latest.and_then(|installed| {
                    if latest > installed {
                        Some(latest.to_string())
                    } else {
                        None
                    }
                })
            });
            content_items.push(version_group_view(
                group,
                default_version,
                search_query,
                update_available,
                schedule,
                operation_queue,
                hovered_version,
            ));
        }
    }

    if !search_query.is_empty() {
        let available = filter_available_versions(remote_versions, search_query);

        if !available.is_empty() {
            content_items.push(Space::new().height(16).into());
            content_items.push(
                text("Available Versions")
                    .size(12)
                    .color(iced::Color::from_rgb8(142, 142, 147))
                    .into(),
            );
            content_items.push(Space::new().height(8).into());

            let available_rows: Vec<Element<Message>> = available
                .iter()
                .map(|v| {
                    available_version_row(
                        v,
                        schedule,
                        operation_queue,
                        &installed_set,
                        hovered_version,
                    )
                })
                .collect();

            content_items.push(
                container(column(available_rows).spacing(4))
                    .style(styles::card_container)
                    .padding(12)
                    .into(),
            );
        }
    }

    if content_items.is_empty() {
        return container(
            column![
                text("No versions found").size(16),
                if search_query.is_empty() {
                    text("Install your first Node.js version by searching above.").size(14)
                } else {
                    text(format!("No versions match '{}'", search_query)).size(14)
                },
            ]
            .spacing(8)
            .align_x(Alignment::Center),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    scrollable(
        column(content_items)
            .spacing(12)
            .padding(iced::Padding::new(0.0).right(32.0)),
    )
    .height(Length::Fill)
    .into()
}

fn filter_group(group: &VersionGroup, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let query_lower = query.to_lowercase();

    if query_lower == "lts" {
        return group.versions.iter().any(|v| v.lts_codename.is_some());
    }

    group.versions.iter().any(|v| {
        let version_str = v.version.to_string();
        version_str.contains(query)
            || v.lts_codename
                .as_ref()
                .map(|c| c.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
    })
}

fn version_group_view<'a>(
    group: &'a VersionGroup,
    default: &'a Option<versi_core::NodeVersion>,
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

fn filter_version(version: &InstalledVersion, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let query_lower = query.to_lowercase();

    if query_lower == "lts" {
        return version.lts_codename.is_some();
    }

    let version_str = version.version.to_string();
    version_str.contains(query)
        || version
            .lts_codename
            .as_ref()
            .map(|c| c.to_lowercase().contains(&query_lower))
            .unwrap_or(false)
}

fn version_item_view<'a>(
    version: &'a InstalledVersion,
    default: &'a Option<versi_core::NodeVersion>,
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

fn available_version_row<'a>(
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

fn format_bytes(bytes: u64) -> String {
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
