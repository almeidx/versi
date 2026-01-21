use std::collections::HashMap;

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};

use fnm_core::{InstalledVersion, NodeVersion, RemoteVersion, VersionGroup};

use crate::message::Message;
use crate::state::EnvironmentState;
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

pub fn view<'a>(
    env: &'a EnvironmentState,
    search_query: &'a str,
    remote_versions: &[RemoteVersion],
) -> Element<'a, Message> {
    let latest_by_major = compute_latest_by_major(remote_versions);
    if env.loading {
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

    let filtered_groups: Vec<&VersionGroup> = env
        .version_groups
        .iter()
        .filter(|group| filter_group(group, search_query))
        .collect();

    if filtered_groups.is_empty() {
        return container(
            column![
                text("No versions found").size(16),
                if search_query.is_empty() {
                    text("Install your first Node.js version to get started.").size(14)
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

    let default_version = &env.default_version;

    let groups: Vec<Element<Message>> = filtered_groups
        .iter()
        .map(|group| {
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
            version_group_view(group, default_version, search_query, update_available)
        })
        .collect();

    scrollable(column(groups).spacing(12).padding([0, 4]))
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
    default: &'a Option<fnm_core::NodeVersion>,
    search_query: &'a str,
    update_available: Option<String>,
) -> Element<'a, Message> {
    let has_lts = group.versions.iter().any(|v| v.lts_codename.is_some());
    let has_default = group
        .versions
        .iter()
        .any(|v| default.as_ref().map(|d| d == &v.version).unwrap_or(false));

    let mut header_row = row![
        text(if group.is_expanded { "▼" } else { "▶" }).size(12),
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

    if has_default {
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

    let header: Element<Message> = if let Some(new_version) = update_available {
        let version_to_install = new_version.clone();
        row![
            header_button,
            Space::new().width(Length::Fill),
            button(container(text(format!("{} available", new_version)).size(10)).padding([2, 6]))
                .on_press(Message::StartInstall(version_to_install))
                .style(styles::update_badge_button)
                .padding(0),
        ]
        .align_y(Alignment::Center)
        .into()
    } else {
        header_button.into()
    };

    if group.is_expanded {
        let filtered_versions: Vec<&InstalledVersion> = group
            .versions
            .iter()
            .filter(|v| filter_version(v, search_query))
            .collect();

        let items: Vec<Element<Message>> = filtered_versions
            .iter()
            .map(|v| version_item_view(v, default))
            .collect();

        container(
            column![
                header,
                container(column(items).spacing(4)).padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 24.0,
                }),
            ]
            .spacing(4),
        )
        .style(styles::card_container)
        .padding(8)
        .into()
    } else {
        container(header)
            .style(styles::card_container)
            .padding(8)
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
    default: &'a Option<fnm_core::NodeVersion>,
) -> Element<'a, Message> {
    let is_default = default
        .as_ref()
        .map(|d| d == &version.version)
        .unwrap_or(false);

    let version_str = version.version.to_string();
    let version_display = version_str.clone();
    let version_for_default = version_str.clone();

    row![
        text(version_display).size(14).width(Length::Fixed(120.0)),
        if let Some(lts) = &version.lts_codename {
            container(text(format!("LTS: {}", lts)).size(11))
                .padding([2, 6])
                .style(styles::badge_lts)
        } else {
            container(Space::new())
        },
        if is_default {
            container(text("default").size(11))
                .padding([2, 6])
                .style(styles::badge_default)
        } else {
            container(Space::new())
        },
        Space::new().width(Length::Fill),
        if let Some(size) = version.disk_size {
            text(format_bytes(size)).size(12)
        } else {
            text("")
        },
        if !is_default {
            button(text("Set Default").size(12))
                .on_press(Message::SetDefault(version_for_default))
                .style(styles::secondary_button)
                .padding([6, 12])
        } else {
            button(text("Default").size(12))
                .style(styles::secondary_button)
                .padding([6, 12])
        },
        button(text("Uninstall").size(12))
            .on_press(Message::RequestUninstall(version_str))
            .style(styles::danger_button)
            .padding([6, 12]),
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
