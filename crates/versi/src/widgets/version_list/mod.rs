mod available;
mod filters;
mod group;
mod item;

use std::collections::{HashMap, HashSet};

use iced::widget::{Space, button, column, container, text};
use iced::{Alignment, Element, Length};

use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion, VersionGroup};
use versi_core::{ReleaseSchedule, VersionMeta};

use crate::message::Message;
use crate::state::{EnvironmentState, OperationQueue, SearchFilter, VersionSecurityFinding};
use crate::theme::styles;
use crate::version_query::{matches_version_query_case_insensitive, passes_release_filters};

use filters::search_available_versions;

pub struct VersionListContext<'a> {
    pub supports_uninstall: bool,
    pub schedule: Option<&'a ReleaseSchedule>,
    pub search_index: Option<&'a crate::version_query::RemoteVersionSearchIndex>,
    pub operation_queue: &'a OperationQueue,
    pub install_progress: &'a HashMap<String, versi_backend::InstallProgress>,
    pub hovered_version: &'a Option<String>,
    pub metadata: Option<&'a HashMap<String, VersionMeta>>,
    pub security_findings: &'a HashMap<String, VersionSecurityFinding>,
    pub installed_set: &'a HashSet<NodeVersion>,
}

fn filter_group(
    group: &VersionGroup,
    query: &str,
    query_lower: &str,
    active_filters: &HashSet<SearchFilter>,
    schedule: Option<&ReleaseSchedule>,
) -> bool {
    if query.is_empty() {
        return true;
    }

    if active_filters.contains(&SearchFilter::NotInstalled) {
        return false;
    }

    if !passes_release_filters(group.major, active_filters, schedule) {
        return false;
    }

    if query_lower == "lts" {
        let has_lts = group.versions.iter().any(|v| v.lts_codename.is_some());
        if !has_lts {
            return false;
        }
        return true;
    }

    let mut version_text = String::with_capacity(16);

    if active_filters.contains(&SearchFilter::Lts) {
        for version in &group.versions {
            if version.lts_codename.is_none() {
                continue;
            }
            if matches_installed_query(version, query, query_lower, &mut version_text) {
                return true;
            }
        }
        return false;
    }

    for version in &group.versions {
        if matches_installed_query(version, query, query_lower, &mut version_text) {
            return true;
        }
    }

    false
}

fn filter_version(
    version: &InstalledVersion,
    query: &str,
    query_lower: &str,
    active_filters: &HashSet<SearchFilter>,
    schedule: Option<&ReleaseSchedule>,
    version_text: &mut String,
) -> bool {
    if query.is_empty() {
        return true;
    }

    let text_match = matches_installed_query(version, query, query_lower, version_text);

    if !text_match {
        return false;
    }

    if active_filters.contains(&SearchFilter::Lts) && version.lts_codename.is_none() {
        return false;
    }
    if active_filters.contains(&SearchFilter::NotInstalled) {
        return false;
    }
    if !passes_release_filters(version.version.major, active_filters, schedule) {
        return false;
    }

    true
}

fn matches_installed_query(
    version: &InstalledVersion,
    query: &str,
    query_lower: &str,
    version_text: &mut String,
) -> bool {
    version.version.write_prefixed_into(version_text);
    matches_version_query_case_insensitive(
        version_text,
        version.lts_codename.as_deref(),
        query,
        query_lower,
    )
}

pub fn view<'a>(
    env: &'a EnvironmentState,
    search_query: &'a str,
    remote_versions: &'a [RemoteVersion],
    latest_by_major: &'a HashMap<u32, NodeVersion>,
    search_results_limit: usize,
    active_filters: &'a HashSet<SearchFilter>,
    ctx: &VersionListContext<'a>,
) -> Element<'a, Message> {
    if let Some(status_view) = loading_or_error_view(env) {
        return status_view;
    }

    let query_lower = search_query.to_lowercase();

    let mut content_items: Vec<Element<Message>> = Vec::new();
    content_items.extend(installed_groups_content(
        env,
        search_query,
        &query_lower,
        latest_by_major,
        active_filters,
        ctx,
    ));
    if let Some(search_results) = search_results_content(
        remote_versions,
        search_query,
        search_results_limit,
        active_filters,
        ctx,
    ) {
        content_items.push(search_results);
    }

    if content_items.is_empty() {
        return empty_versions_view(search_query);
    }

    column(content_items).spacing(12).width(Length::Fill).into()
}

fn loading_or_error_view(env: &EnvironmentState) -> Option<Element<'_, Message>> {
    if env.loading && env.installed_versions.is_empty() {
        return Some(
            container(
                column![text("Loading versions...").size(16),]
                    .spacing(8)
                    .align_x(Alignment::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .height(Length::Fill)
            .into(),
        );
    }

    env.error.as_ref().map(|error| {
        container(
            column![
                text("Error loading versions").size(16),
                text(error.to_string()).size(14),
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
        .into()
    })
}

fn installed_groups_content<'a>(
    env: &'a EnvironmentState,
    search_query: &'a str,
    query_lower: &str,
    latest_by_major: &'a HashMap<u32, NodeVersion>,
    active_filters: &'a HashSet<SearchFilter>,
    ctx: &VersionListContext<'a>,
) -> Vec<Element<'a, Message>> {
    let filtered_groups: Vec<&VersionGroup> = env
        .version_groups
        .iter()
        .filter(|group| {
            filter_group(
                group,
                search_query,
                query_lower,
                active_filters,
                ctx.schedule,
            )
        })
        .collect();

    if filtered_groups.is_empty() || !search_query.is_empty() {
        return Vec::new();
    }

    filtered_groups
        .iter()
        .map(|group| {
            let update_available = update_available_for_group(group, latest_by_major);
            group::version_group_view(
                group,
                env.default_version.as_ref(),
                search_query,
                query_lower,
                update_available,
                active_filters,
                ctx,
            )
        })
        .collect()
}

fn update_available_for_group(
    group: &VersionGroup,
    latest_by_major: &HashMap<u32, NodeVersion>,
) -> Option<String> {
    let installed_latest = group.versions.iter().map(|version| &version.version).max();
    latest_by_major.get(&group.major).and_then(|latest| {
        installed_latest.and_then(|installed| {
            if latest > installed {
                Some(latest.to_string())
            } else {
                None
            }
        })
    })
}

fn search_results_content<'a>(
    remote_versions: &'a [RemoteVersion],
    search_query: &'a str,
    search_results_limit: usize,
    active_filters: &'a HashSet<SearchFilter>,
    ctx: &VersionListContext<'a>,
) -> Option<Element<'a, Message>> {
    if search_query.is_empty() {
        return None;
    }

    let search = search_available_versions(
        remote_versions,
        ctx.search_index,
        search_query,
        search_results_limit,
        active_filters,
        ctx.installed_set,
        ctx.schedule,
    );

    if search.versions.is_empty() {
        return None;
    }

    let mut card_items: Vec<Element<Message>> = Vec::new();
    if search.alias_resolved {
        card_items.push(
            text(format!("\"{search_query}\" resolves to:"))
                .size(12)
                .color(crate::theme::tokens::TEXT_MUTED)
                .into(),
        );
        card_items.push(Space::new().height(4).into());
    }
    for version in &search.versions {
        card_items.push(available::available_version_row(version, ctx));
    }

    Some(
        container(column(card_items).spacing(4))
            .style(styles::card_container)
            .padding(12)
            .into(),
    )
}

fn empty_versions_view(search_query: &str) -> Element<'_, Message> {
    container(
        column![
            text("No versions found").size(16),
            if search_query.is_empty() {
                text("Install your first Node.js version by searching above.").size(14)
            } else {
                text(format!("No versions match '{search_query}'")).size(14)
            },
        ]
        .spacing(8)
        .align_x(Alignment::Center),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::update_available_for_group;
    use crate::state::SearchFilter;
    use crate::version_query::{matches_version_query, passes_release_filters};
    use versi_backend::{InstalledVersion, NodeVersion, VersionGroup};

    fn installed(version: &str) -> InstalledVersion {
        InstalledVersion {
            version: version.parse().expect("test version should parse"),
            is_default: false,
            lts_codename: Some("Iron".to_string()),
            install_date: None,
            disk_size: None,
        }
    }

    fn schedule_with_eol_major(eol_major: u32) -> versi_core::ReleaseSchedule {
        serde_json::from_value(serde_json::json!({
            "versions": {
                format!("{eol_major}"): {
                    "start": "2020-01-01",
                    "end": "2021-01-01"
                },
                "22": {
                    "start": "2024-04-23",
                    "lts": "2024-10-29",
                    "maintenance": "2026-10-20",
                    "end": "2027-04-30",
                    "codename": "Jod"
                }
            }
        }))
        .expect("schedule fixture should deserialize")
    }

    #[test]
    fn matches_query_handles_versions_and_lts_codenames() {
        let version_text = NodeVersion::new(22, 11, 0).to_string();
        assert!(matches_version_query(
            &version_text,
            Some("jod"),
            "22",
            "22"
        ));
        assert!(matches_version_query(
            &version_text,
            Some("jod"),
            "jod",
            "jod"
        ));
        assert!(matches_version_query(
            &version_text,
            Some("jod"),
            "lts",
            "lts"
        ));
        assert!(!matches_version_query(&version_text, None, "lts", "lts"));
    }

    #[test]
    fn release_filters_respect_eol_and_active_flags() {
        let schedule = schedule_with_eol_major(20);
        assert!(passes_release_filters(22, &HashSet::new(), Some(&schedule)));
        assert!(!passes_release_filters(
            22,
            &HashSet::from([SearchFilter::Eol]),
            Some(&schedule)
        ));
        assert!(!passes_release_filters(
            20,
            &HashSet::from([SearchFilter::Active]),
            Some(&schedule)
        ));
        assert!(passes_release_filters(
            20,
            &HashSet::from([SearchFilter::Eol]),
            Some(&schedule)
        ));
    }

    #[test]
    fn update_available_for_group_returns_newer_version_only() {
        let group = VersionGroup::from_versions(&[installed("v22.1.0"), installed("v22.0.0")])
            .into_iter()
            .find(|g| g.major == 22)
            .expect("major group should exist");

        let latest = std::collections::HashMap::from([
            (22, NodeVersion::new(22, 2, 0)),
            (20, NodeVersion::new(20, 11, 0)),
        ]);
        assert_eq!(
            update_available_for_group(&group, &latest),
            Some("v22.2.0".to_string())
        );

        let latest_equal = std::collections::HashMap::from([(22, NodeVersion::new(22, 1, 0))]);
        assert_eq!(update_available_for_group(&group, &latest_equal), None);
    }
}
