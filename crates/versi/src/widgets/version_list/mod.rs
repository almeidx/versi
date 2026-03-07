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

    use super::{filter_group, filter_version, update_available_for_group};
    use crate::state::SearchFilter;
    use crate::version_query::{matches_version_query, passes_release_filters};
    use versi_backend::{InstalledVersion, NodeVersion, VersionGroup};

    fn installed(version: &str) -> InstalledVersion {
        InstalledVersion {
            version: version.parse().expect("test version should parse"),
            is_default: false,
            lts_codename: Some("Iron".to_string()),
            disk_size: None,
        }
    }

    fn installed_no_lts(version: &str) -> InstalledVersion {
        InstalledVersion {
            version: version.parse().expect("test version should parse"),
            is_default: false,
            lts_codename: None,
            disk_size: None,
        }
    }

    fn installed_with_lts(version: &str, codename: &str) -> InstalledVersion {
        InstalledVersion {
            version: version.parse().expect("test version should parse"),
            is_default: false,
            lts_codename: Some(codename.to_string()),
            disk_size: None,
        }
    }

    use crate::test_fixtures::schedule_with_eol_major;

    fn make_group(versions: &[InstalledVersion]) -> VersionGroup {
        VersionGroup::from_versions(versions)
            .into_iter()
            .next()
            .expect("at least one group")
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

    // -- filter_group tests --

    #[test]
    fn filter_group_passes_all_groups_on_empty_query() {
        let group = make_group(&[installed_no_lts("v22.1.0")]);
        assert!(filter_group(&group, "", "", &HashSet::new(), None));
    }

    #[test]
    fn filter_group_rejects_when_not_installed_filter_is_active() {
        let group = make_group(&[installed("v22.1.0")]);
        let filters = HashSet::from([SearchFilter::NotInstalled]);
        assert!(!filter_group(&group, "22", "22", &filters, None));
    }

    #[test]
    fn filter_group_rejects_when_release_filter_excludes_major() {
        let group = make_group(&[installed("v20.11.0")]);
        let filters = HashSet::from([SearchFilter::Active]);
        let schedule = schedule_with_eol_major(20);
        assert!(!filter_group(&group, "20", "20", &filters, Some(&schedule)));
    }

    #[test]
    fn filter_group_lts_query_matches_groups_with_lts_versions() {
        let group_lts = make_group(&[installed_with_lts("v22.1.0", "Jod")]);
        assert!(filter_group(
            &group_lts,
            "lts",
            "lts",
            &HashSet::new(),
            None
        ));

        let group_no_lts = make_group(&[installed_no_lts("v23.1.0")]);
        assert!(!filter_group(
            &group_no_lts,
            "lts",
            "lts",
            &HashSet::new(),
            None
        ));
    }

    #[test]
    fn filter_group_matches_version_number_substring() {
        let group = make_group(&[installed_no_lts("v22.3.0"), installed_no_lts("v22.1.0")]);
        assert!(filter_group(&group, "22.3", "22.3", &HashSet::new(), None));
        assert!(!filter_group(&group, "99", "99", &HashSet::new(), None));
    }

    #[test]
    fn filter_group_with_lts_filter_only_matches_lts_versions() {
        let group = make_group(&[
            installed_with_lts("v22.2.0", "Jod"),
            installed_no_lts("v22.1.0"),
        ]);
        let filters = HashSet::from([SearchFilter::Lts]);

        assert!(filter_group(&group, "22.2", "22.2", &filters, None));
        assert!(!filter_group(&group, "22.1", "22.1", &filters, None));
    }

    #[test]
    fn filter_group_matches_lts_codename_case_insensitive() {
        let group = make_group(&[installed_with_lts("v22.1.0", "Jod")]);
        assert!(filter_group(&group, "jod", "jod", &HashSet::new(), None));
        assert!(filter_group(&group, "Jod", "jod", &HashSet::new(), None));
    }

    #[test]
    fn filter_group_eol_filter_restricts_to_eol_majors() {
        let schedule = schedule_with_eol_major(20);
        let group_22 = make_group(&[installed("v22.1.0")]);
        let group_20 = make_group(&[installed("v20.11.0")]);
        let filters = HashSet::from([SearchFilter::Eol]);

        assert!(!filter_group(
            &group_22,
            "v",
            "v",
            &filters,
            Some(&schedule)
        ));
        assert!(filter_group(&group_20, "v", "v", &filters, Some(&schedule)));
    }

    // -- filter_version tests --

    #[test]
    fn filter_version_passes_on_empty_query() {
        let version = installed("v22.1.0");
        let mut buf = String::new();
        assert!(filter_version(
            &version,
            "",
            "",
            &HashSet::new(),
            None,
            &mut buf
        ));
    }

    #[test]
    fn filter_version_matches_version_text() {
        let version = installed_no_lts("v22.3.0");
        let mut buf = String::new();
        assert!(filter_version(
            &version,
            "22.3",
            "22.3",
            &HashSet::new(),
            None,
            &mut buf,
        ));
        assert!(!filter_version(
            &version,
            "20",
            "20",
            &HashSet::new(),
            None,
            &mut buf,
        ));
    }

    #[test]
    fn filter_version_rejects_non_lts_when_lts_filter_active() {
        let version = installed_no_lts("v23.1.0");
        let mut buf = String::new();
        let filters = HashSet::from([SearchFilter::Lts]);
        assert!(!filter_version(
            &version, "23", "23", &filters, None, &mut buf,
        ));
    }

    #[test]
    fn filter_version_passes_lts_version_when_lts_filter_active() {
        let version = installed_with_lts("v22.1.0", "Jod");
        let mut buf = String::new();
        let filters = HashSet::from([SearchFilter::Lts]);
        assert!(filter_version(
            &version, "22", "22", &filters, None, &mut buf,
        ));
    }

    #[test]
    fn filter_version_rejects_with_not_installed_filter() {
        let version = installed("v22.1.0");
        let mut buf = String::new();
        let filters = HashSet::from([SearchFilter::NotInstalled]);
        assert!(!filter_version(
            &version, "22", "22", &filters, None, &mut buf,
        ));
    }

    #[test]
    fn filter_version_respects_eol_release_filter() {
        let schedule = schedule_with_eol_major(20);
        let active_version = installed("v22.1.0");
        let eol_version = installed("v20.11.0");
        let mut buf = String::new();
        let eol_filters = HashSet::from([SearchFilter::Eol]);

        assert!(!filter_version(
            &active_version,
            "v",
            "v",
            &eol_filters,
            Some(&schedule),
            &mut buf,
        ));
        assert!(filter_version(
            &eol_version,
            "v",
            "v",
            &eol_filters,
            Some(&schedule),
            &mut buf,
        ));
    }

    #[test]
    fn filter_version_rejects_text_mismatch_even_with_passing_filters() {
        let version = installed_with_lts("v22.1.0", "Jod");
        let mut buf = String::new();
        let filters = HashSet::from([SearchFilter::Lts]);
        assert!(!filter_version(
            &version, "99", "99", &filters, None, &mut buf,
        ));
    }

    #[test]
    fn filter_version_matches_lts_codename() {
        let version = installed_with_lts("v22.1.0", "Jod");
        let mut buf = String::new();
        assert!(filter_version(
            &version,
            "Jod",
            "jod",
            &HashSet::new(),
            None,
            &mut buf,
        ));
    }

    #[test]
    fn filter_version_buffer_is_reused_across_calls() {
        let v1 = installed_no_lts("v22.1.0");
        let v2 = installed_no_lts("v20.11.0");
        let mut buf = String::with_capacity(16);
        let initial_ptr = buf.as_ptr();

        filter_version(&v1, "22", "22", &HashSet::new(), None, &mut buf);
        filter_version(&v2, "20", "20", &HashSet::new(), None, &mut buf);

        assert_eq!(buf.as_ptr(), initial_ptr);
    }
}
