use std::collections::HashMap;

use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

use fnm_core::{ReleaseSchedule, RemoteVersion};

use crate::message::Message;
use crate::state::{InstallModalState, MainState};
use crate::theme::styles;

fn filter_latest_patches(versions: &[RemoteVersion]) -> Vec<&RemoteVersion> {
    let mut latest_by_minor: HashMap<(u32, u32), &RemoteVersion> = HashMap::new();

    for version in versions {
        let key = (version.version.major, version.version.minor);

        latest_by_minor
            .entry(key)
            .and_modify(|existing| {
                if version.version.patch > existing.version.patch {
                    *existing = version;
                }
            })
            .or_insert(version);
    }

    let mut result: Vec<&RemoteVersion> = latest_by_minor.into_values().collect();
    result.sort_by(|a, b| b.version.cmp(&a.version));
    result
}

fn group_by_major<'a>(
    versions: &[&'a RemoteVersion],
    max_per_group: usize,
    schedule: Option<&ReleaseSchedule>,
) -> Vec<(u32, Vec<&'a RemoteVersion>)> {
    let mut by_major: HashMap<u32, Vec<&'a RemoteVersion>> = HashMap::new();

    for version in versions {
        let major = version.version.major;
        let is_active = schedule.map(|s| s.is_active(major)).unwrap_or(true);
        if is_active {
            by_major.entry(major).or_default().push(*version);
        }
    }

    let mut groups: Vec<(u32, Vec<&RemoteVersion>)> = by_major
        .into_iter()
        .map(|(major, mut vers)| {
            vers.sort_by(|a, b| b.version.cmp(&a.version));
            vers.truncate(max_per_group);
            (major, vers)
        })
        .collect();

    groups.sort_by(|a, b| b.0.cmp(&a.0));
    groups
}

fn get_recommended_versions<'a>(
    versions: &'a [RemoteVersion],
    schedule: Option<&ReleaseSchedule>,
) -> Vec<&'a RemoteVersion> {
    let mut latest_by_major: HashMap<u32, &RemoteVersion> = HashMap::new();

    for version in versions {
        let major = version.version.major;
        let is_active = schedule.map(|s| s.is_active(major)).unwrap_or(true);

        if !is_active {
            continue;
        }

        latest_by_major
            .entry(major)
            .and_modify(|existing| {
                if version.version > existing.version {
                    *existing = version;
                }
            })
            .or_insert(version);
    }

    let mut active_majors: Vec<u32> = latest_by_major.keys().copied().collect();
    active_majors.sort_by(|a, b| b.cmp(a));

    active_majors
        .into_iter()
        .take(8)
        .filter_map(|major| latest_by_major.get(&major).copied())
        .collect()
}

pub fn view<'a>(
    modal_state: &'a InstallModalState,
    _main_state: &'a MainState,
) -> Element<'a, Message> {
    let header = row![
        text("Install Node.js").size(20),
        Space::new().width(Length::Fill),
        button(text("×").size(16))
            .on_press(Message::CloseModal)
            .style(styles::ghost_button)
            .padding([4, 10]),
    ]
    .align_y(Alignment::Center);

    let search = text_input(
        "Search versions (e.g., '20', 'lts')",
        &modal_state.search_query,
    )
    .on_input(Message::InstallModalSearchChanged)
    .padding(14)
    .size(14)
    .style(styles::search_input);

    let content: Element<Message> = if modal_state.loading
        && modal_state.filtered_versions.is_empty()
    {
        container(text("Loading available versions...").size(14))
            .center_x(Length::Fill)
            .padding(24)
            .into()
    } else if modal_state.filtered_versions.is_empty() {
        container(
            column![
                text("No versions found").size(14),
                if !modal_state.search_query.is_empty() {
                    text(format!("No versions match '{}'", modal_state.search_query)).size(12)
                } else {
                    text("").size(12)
                },
            ]
            .spacing(4)
            .align_x(Alignment::Center),
        )
        .center_x(Length::Fill)
        .padding(24)
        .into()
    } else if modal_state.search_query.is_empty() {
        let recommended = get_recommended_versions(
            &modal_state.filtered_versions,
            modal_state.schedule.as_ref(),
        );

        let mut version_items: Vec<Element<Message>> = Vec::new();
        version_items.push(
            text("Recommended Versions")
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147))
                .into(),
        );
        version_items.push(Space::new().height(8).into());

        for version in recommended {
            version_items.push(version_row(version));
        }

        version_items.push(Space::new().height(16).into());
        version_items.push(
            text("Search for other versions above")
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147))
                .into(),
        );

        scrollable(column(version_items).spacing(4))
            .height(Length::Fixed(300.0))
            .into()
    } else {
        let filtered = filter_latest_patches(&modal_state.filtered_versions);

        let lts_versions: Vec<&RemoteVersion> = filtered
            .iter()
            .filter(|v| v.lts_codename.is_some())
            .copied()
            .collect();

        let other_versions: Vec<&RemoteVersion> = filtered
            .iter()
            .filter(|v| v.lts_codename.is_none())
            .copied()
            .collect();

        let mut version_items: Vec<Element<Message>> = Vec::new();

        if !lts_versions.is_empty() {
            let grouped_lts = group_by_major(&lts_versions, 5, modal_state.schedule.as_ref());

            version_items.push(
                text("LTS Versions")
                    .size(12)
                    .color(iced::Color::from_rgb8(142, 142, 147))
                    .into(),
            );
            version_items.push(Space::new().height(4).into());

            for (major, versions) in grouped_lts {
                let codename = versions
                    .first()
                    .and_then(|v| v.lts_codename.as_ref())
                    .map(|c| format!(" ({})", c))
                    .unwrap_or_default();
                version_items.push(
                    text(format!("Node {}.x{}", major, codename))
                        .size(11)
                        .color(iced::Color::from_rgb8(142, 142, 147))
                        .into(),
                );
                for version in versions {
                    version_items.push(version_row(version));
                }
                version_items.push(Space::new().height(8).into());
            }
        }

        if !other_versions.is_empty() {
            let grouped_other = group_by_major(&other_versions, 5, modal_state.schedule.as_ref());

            version_items.push(
                text("Other Versions")
                    .size(12)
                    .color(iced::Color::from_rgb8(142, 142, 147))
                    .into(),
            );
            version_items.push(Space::new().height(4).into());

            for (major, versions) in grouped_other {
                version_items.push(
                    text(format!("Node {}.x", major))
                        .size(11)
                        .color(iced::Color::from_rgb8(142, 142, 147))
                        .into(),
                );
                for version in versions {
                    version_items.push(version_row(version));
                }
                version_items.push(Space::new().height(8).into());
            }
        }

        scrollable(column(version_items).spacing(4))
            .height(Length::Fixed(300.0))
            .into()
    };

    column![
        header,
        Space::new().height(16),
        search,
        Space::new().height(16),
        content,
    ]
    .spacing(8)
    .width(Length::Fill)
    .into()
}

fn version_row<'a>(version: &'a RemoteVersion) -> Element<'a, Message> {
    let version_str = version.version.to_string();
    let version_display = version_str.clone();
    let version_for_changelog = version_str.clone();

    row![
        text(version_display).size(14).width(Length::Fixed(100.0)),
        if let Some(lts) = &version.lts_codename {
            container(text(lts.clone()).size(11))
                .padding([2, 6])
                .style(styles::badge_lts)
        } else {
            container(Space::new())
        },
        Space::new().width(Length::Fill),
        button(text("Changelog").size(11))
            .on_press(Message::OpenChangelog(version_for_changelog))
            .style(styles::ghost_button)
            .padding([4, 8]),
        button(text("Install").size(12))
            .on_press(Message::StartInstall(version_str))
            .style(styles::primary_button)
            .padding([6, 12]),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([4, 8])
    .into()
}
