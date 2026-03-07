use std::collections::HashSet;

use versi_backend::{NodeVersion, RemoteVersion};
use versi_core::ReleaseSchedule;

use crate::state::SearchFilter;
use crate::version_query;

pub(super) fn search_available_versions<'a>(
    versions: &'a [RemoteVersion],
    search_index: Option<&version_query::RemoteVersionSearchIndex>,
    query: &str,
    limit: usize,
    active_filters: &HashSet<SearchFilter>,
    installed_set: &HashSet<NodeVersion>,
    schedule: Option<&ReleaseSchedule>,
) -> version_query::AvailableVersionSearch<'a> {
    version_query::search_available_versions_with_index(
        versions,
        search_index,
        query,
        limit,
        active_filters,
        installed_set,
        schedule,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::search_available_versions;
    use crate::state::SearchFilter;

    use crate::test_fixtures::{remote, schedule_with_eol_major};

    #[test]
    fn resolve_alias_latest_returns_highest_version() {
        let versions = vec![remote("v20.11.0", None), remote("v22.1.0", Some("Jod"))];

        let resolved = crate::version_query::resolve_alias_with_index(&versions, None, "latest")
            .expect("latest alias should resolve");

        assert_eq!(resolved.version.to_string(), "v22.1.0");
    }

    #[test]
    fn resolve_alias_lts_codename_is_case_insensitive() {
        let versions = vec![
            remote("v20.11.0", Some("Iron")),
            remote("v20.12.0", Some("Iron")),
            remote("v22.1.0", Some("Jod")),
        ];

        let resolved = crate::version_query::resolve_alias_with_index(&versions, None, "lts/iron")
            .expect("lts codename should resolve");

        assert_eq!(resolved.version.to_string(), "v20.12.0");
    }

    #[test]
    fn non_lts_query_honors_limit_argument() {
        let versions = vec![
            remote("v22.3.0", None),
            remote("v22.2.0", None),
            remote("v22.1.0", None),
        ];

        let filtered = search_available_versions(
            &versions,
            None,
            "v22",
            2,
            &HashSet::new(),
            &HashSet::new(),
            None,
        )
        .versions;

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].version.to_string(), "v22.3.0");
        assert_eq!(filtered[1].version.to_string(), "v22.2.0");
    }

    #[test]
    fn active_filters_installed_and_eol_are_applied() {
        let versions = vec![
            remote("v22.1.0", Some("Jod")),
            remote("v20.11.0", Some("Iron")),
        ];
        let installed = HashSet::from([versi_backend::NodeVersion::new(20, 11, 0)]);
        let filters = HashSet::from([SearchFilter::Installed, SearchFilter::Eol]);
        let schedule = schedule_with_eol_major(20);

        let filtered = search_available_versions(
            &versions,
            None,
            "v",
            10,
            &filters,
            &installed,
            Some(&schedule),
        )
        .versions;

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].version.to_string(), "v20.11.0");
    }

    #[test]
    fn search_marks_alias_resolution_for_ui_hints() {
        let versions = vec![remote("v20.11.0", None), remote("v22.1.0", Some("Jod"))];
        let search = search_available_versions(
            &versions,
            None,
            "stable",
            10,
            &HashSet::new(),
            &HashSet::new(),
            None,
        );

        assert!(search.alias_resolved);
        assert_eq!(search.versions.len(), 1);
    }

    #[test]
    fn alias_resolution_respects_not_installed_filter() {
        let versions = vec![remote("v22.1.0", Some("Jod")), remote("v20.11.0", None)];
        let installed = HashSet::from([versi_backend::NodeVersion::new(22, 1, 0)]);
        let filters = HashSet::from([SearchFilter::NotInstalled]);

        let search =
            search_available_versions(&versions, None, "stable", 10, &filters, &installed, None);

        assert!(search.alias_resolved);
        assert!(search.versions.is_empty());
    }

    #[test]
    fn installed_filter_is_applied_before_limit() {
        let versions = vec![
            remote("v22.3.0", None),
            remote("v22.2.0", None),
            remote("v22.1.0", None),
        ];
        let installed = HashSet::from([versi_backend::NodeVersion::new(22, 2, 0)]);
        let filters = HashSet::from([SearchFilter::Installed]);

        let search =
            search_available_versions(&versions, None, "v22", 1, &filters, &installed, None);

        assert_eq!(search.versions.len(), 1);
        assert_eq!(search.versions[0].version.to_string(), "v22.2.0");
    }
}
