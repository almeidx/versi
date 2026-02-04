mod available;
mod filters;
mod group;
mod item;

use std::collections::HashMap;

use iced::widget::{Space, button, column, container, scrollable, text};
use iced::{Alignment, Element, Length};

use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion, VersionGroup};
use versi_core::ReleaseSchedule;

use crate::message::Message;
use crate::state::{EnvironmentState, OperationQueue};
use crate::theme::styles;

use filters::{filter_available_versions, resolve_alias};

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

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    env: &'a EnvironmentState,
    search_query: &'a str,
    remote_versions: &'a [RemoteVersion],
    latest_by_major: &'a HashMap<u32, NodeVersion>,
    schedule: Option<&'a ReleaseSchedule>,
    operation_queue: &'a OperationQueue,
    hovered_version: &'a Option<String>,
    search_results_limit: usize,
) -> Element<'a, Message> {
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

    let filtered_groups: Vec<&VersionGroup> = env
        .version_groups
        .iter()
        .filter(|g| filter_group(g, search_query))
        .collect();

    let default_version = &env.default_version;

    let mut content_items: Vec<Element<Message>> = Vec::new();

    if !filtered_groups.is_empty() && search_query.is_empty() {
        for g in &filtered_groups {
            let installed_latest = g.versions.iter().map(|v| &v.version).max();
            let update_available = latest_by_major.get(&g.major).and_then(|latest| {
                installed_latest.and_then(|installed| {
                    if latest > installed {
                        Some(latest.to_string())
                    } else {
                        None
                    }
                })
            });
            content_items.push(group::version_group_view(
                g,
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
        let alias_resolved = resolve_alias(remote_versions, search_query);
        let available_list =
            filter_available_versions(remote_versions, search_query, search_results_limit);

        if !available_list.is_empty() {
            let mut card_items: Vec<Element<Message>> = Vec::new();

            if alias_resolved.is_some() {
                card_items.push(
                    text(format!("\"{}\" resolves to:", search_query))
                        .size(12)
                        .color(iced::Color::from_rgb8(142, 142, 147))
                        .into(),
                );
                card_items.push(Space::new().height(4).into());
            }

            for v in &available_list {
                card_items.push(available::available_version_row(
                    v,
                    schedule,
                    operation_queue,
                    &env.installed_set,
                    hovered_version,
                ));
            }

            content_items.push(
                container(column(card_items).spacing(4))
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
