use std::collections::{HashMap, HashSet};

use versi_backend::{NodeVersion, RemoteVersion};
use versi_core::ReleaseSchedule;

use crate::state::SearchFilter;

pub(crate) struct AvailableVersionSearch<'a> {
    pub(crate) versions: Vec<&'a RemoteVersion>,
    pub(crate) alias_resolved: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RemoteVersionSearchIndex {
    entries: Vec<RemoteVersionSearchEntry>,
}

#[derive(Debug, Clone)]
struct RemoteVersionSearchEntry {
    version_text: String,
    lts_codename_lower: Option<String>,
}

const LATEST_BY_MAJOR_CAPACITY: usize = 32;
const LATEST_BY_MINOR_CAPACITY: usize = 192;

impl RemoteVersionSearchIndex {
    #[must_use]
    pub(crate) fn from_versions(versions: &[RemoteVersion]) -> Self {
        let entries = versions
            .iter()
            .map(|version| RemoteVersionSearchEntry {
                version_text: version.version.to_string(),
                lts_codename_lower: version.lts_codename.as_deref().map(str::to_lowercase),
            })
            .collect();

        Self { entries }
    }

    fn as_complete_entries_for<'a>(
        &'a self,
        versions: &[RemoteVersion],
    ) -> Option<&'a [RemoteVersionSearchEntry]> {
        (self.entries.len() == versions.len()).then_some(self.entries.as_slice())
    }
}

pub(crate) fn matches_version_query(
    version_text: &str,
    lts_codename_lower: Option<&str>,
    query: &str,
    query_lower: &str,
) -> bool {
    if query_lower == "lts" {
        return lts_codename_lower.is_some();
    }

    version_text.contains(query)
        || lts_codename_lower.is_some_and(|codename| codename.contains(query_lower))
}

pub(crate) fn matches_version_query_case_insensitive(
    version_text: &str,
    lts_codename: Option<&str>,
    query: &str,
    query_lower: &str,
) -> bool {
    if query_lower == "lts" {
        return lts_codename.is_some();
    }

    version_text.contains(query)
        || lts_codename.is_some_and(|codename| contains_case_insensitive(codename, query_lower))
}

pub(crate) fn passes_release_filters(
    major: u32,
    active_filters: &HashSet<SearchFilter>,
    schedule: Option<&ReleaseSchedule>,
) -> bool {
    if active_filters.contains(&SearchFilter::Eol) {
        let is_eol = schedule.is_some_and(|s| !s.is_active(major));
        if !is_eol {
            return false;
        }
    }

    if active_filters.contains(&SearchFilter::Active) {
        let is_active = schedule.is_none_or(|s| s.is_active(major));
        if !is_active {
            return false;
        }
    }

    true
}

pub(crate) fn resolve_alias_with_index<'a>(
    versions: &'a [RemoteVersion],
    search_index: Option<&RemoteVersionSearchIndex>,
    query: &str,
) -> Option<&'a RemoteVersion> {
    let query_lower = query.to_lowercase();
    let index_entries = search_index.and_then(|index| index.as_complete_entries_for(versions));

    match query_lower.as_str() {
        "latest" | "stable" | "current" => versions.iter().max_by_key(|v| &v.version),
        "lts/*" => versions
            .iter()
            .filter(|v| v.lts_codename.is_some())
            .max_by_key(|v| &v.version),
        q if q.starts_with("lts/") => {
            let codename = &q[4..];
            if let Some(entries) = index_entries {
                versions
                    .iter()
                    .zip(entries.iter())
                    .filter(|(_, entry)| {
                        entry
                            .lts_codename_lower
                            .as_deref()
                            .is_some_and(|lts| lts == codename)
                    })
                    .map(|(version, _)| version)
                    .max_by_key(|version| &version.version)
            } else {
                versions
                    .iter()
                    .filter(|v| {
                        v.lts_codename
                            .as_ref()
                            .is_some_and(|c| c.to_lowercase() == codename)
                    })
                    .max_by_key(|v| &v.version)
            }
        }
        _ => None,
    }
}

pub(crate) fn search_available_versions_with_index<'a>(
    versions: &'a [RemoteVersion],
    search_index: Option<&RemoteVersionSearchIndex>,
    query: &str,
    limit: usize,
    active_filters: &HashSet<SearchFilter>,
    installed_set: &HashSet<NodeVersion>,
    schedule: Option<&ReleaseSchedule>,
) -> AvailableVersionSearch<'a> {
    let query_lower = query.to_lowercase();
    let index_entries = search_index.and_then(|index| index.as_complete_entries_for(versions));

    if let Some(resolved) = resolve_alias_with_index(versions, search_index, query) {
        let filtered = if matches_active_filters(resolved, active_filters, installed_set, schedule)
        {
            vec![resolved]
        } else {
            Vec::new()
        };
        return AvailableVersionSearch {
            versions: filtered,
            alias_resolved: true,
        };
    }

    let mut result = if query_lower == "lts" {
        latest_by_major(versions.iter().filter(|v| v.lts_codename.is_some()))
    } else if let Some(entries) = index_entries {
        latest_by_minor(
            versions
                .iter()
                .zip(entries.iter())
                .filter(|(_, entry)| {
                    matches_version_query(
                        &entry.version_text,
                        entry.lts_codename_lower.as_deref(),
                        query,
                        &query_lower,
                    )
                })
                .map(|(version, _)| version),
        )
    } else {
        let mut filtered: Vec<&RemoteVersion> = Vec::with_capacity(versions.len());
        let mut version_text = String::with_capacity(16);
        for version in versions {
            version.version.write_prefixed_into(&mut version_text);
            if matches_version_query_case_insensitive(
                &version_text,
                version.lts_codename.as_deref(),
                query,
                &query_lower,
            ) {
                filtered.push(version);
            }
        }
        latest_by_minor(filtered.into_iter())
    };
    apply_active_filters(&mut result, active_filters, installed_set, schedule);
    result.truncate(limit);

    AvailableVersionSearch {
        versions: result,
        alias_resolved: false,
    }
}

fn latest_by_major<'a>(
    versions: impl Iterator<Item = &'a RemoteVersion>,
) -> Vec<&'a RemoteVersion> {
    let mut latest_by_major: HashMap<u32, &RemoteVersion> =
        HashMap::with_capacity(LATEST_BY_MAJOR_CAPACITY);

    for version in versions {
        latest_by_major
            .entry(version.version.major)
            .and_modify(|existing| {
                if version.version > existing.version {
                    *existing = version;
                }
            })
            .or_insert(version);
    }

    let mut result: Vec<&RemoteVersion> = latest_by_major.into_values().collect();
    result.sort_by_key(|v| std::cmp::Reverse(v.version));
    result
}

fn latest_by_minor<'a>(
    versions: impl Iterator<Item = &'a RemoteVersion>,
) -> Vec<&'a RemoteVersion> {
    let mut latest_by_minor: HashMap<(u32, u32), &RemoteVersion> =
        HashMap::with_capacity(LATEST_BY_MINOR_CAPACITY);

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
    result.sort_by_key(|v| std::cmp::Reverse(v.version));
    result
}

fn apply_active_filters(
    versions: &mut Vec<&RemoteVersion>,
    active_filters: &HashSet<SearchFilter>,
    installed_set: &HashSet<NodeVersion>,
    schedule: Option<&ReleaseSchedule>,
) {
    if active_filters.is_empty() {
        return;
    }

    versions
        .retain(|version| matches_active_filters(version, active_filters, installed_set, schedule));
}

fn matches_active_filters(
    version: &RemoteVersion,
    active_filters: &HashSet<SearchFilter>,
    installed_set: &HashSet<NodeVersion>,
    schedule: Option<&ReleaseSchedule>,
) -> bool {
    if active_filters.contains(&SearchFilter::Lts) && version.lts_codename.is_none() {
        return false;
    }

    if active_filters.contains(&SearchFilter::Installed)
        && !installed_set.contains(&version.version)
    {
        return false;
    }

    if active_filters.contains(&SearchFilter::NotInstalled)
        && installed_set.contains(&version.version)
    {
        return false;
    }

    passes_release_filters(version.version.major, active_filters, schedule)
}

fn contains_case_insensitive(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }

    if haystack.is_ascii() && needle_lower.is_ascii() {
        let haystack_bytes = haystack.as_bytes();
        let needle_bytes = needle_lower.as_bytes();
        if needle_bytes.len() > haystack_bytes.len() {
            return false;
        }
        return haystack_bytes
            .windows(needle_bytes.len())
            .any(|window| window.eq_ignore_ascii_case(needle_bytes));
    }

    haystack.to_lowercase().contains(needle_lower)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    use super::{
        RemoteVersionSearchIndex, resolve_alias_with_index, search_available_versions_with_index,
    };
    use crate::state::SearchFilter;

    use crate::test_fixtures::{remote, schedule_with_eol_major};

    #[test]
    fn alias_latest_resolves_to_highest_version() {
        let versions = vec![remote("v20.11.0", None), remote("v22.1.0", Some("Jod"))];
        let resolved =
            resolve_alias_with_index(&versions, None, "latest").expect("alias should resolve");
        assert_eq!(resolved.version.to_string(), "v22.1.0");
    }

    #[test]
    fn indexed_alias_resolution_matches_unindexed_behavior() {
        let versions = vec![
            remote("v20.11.0", Some("Iron")),
            remote("v20.12.0", Some("Iron")),
            remote("v22.1.0", Some("Jod")),
        ];
        let search_index = RemoteVersionSearchIndex::from_versions(&versions);

        let unindexed = resolve_alias_with_index(&versions, None, "lts/iron")
            .map(|version| version.version.to_string())
            .expect("alias should resolve");
        let indexed = resolve_alias_with_index(&versions, Some(&search_index), "lts/iron")
            .map(|version| version.version.to_string())
            .expect("indexed alias should resolve");

        assert_eq!(indexed, unindexed);
        assert_eq!(indexed, "v20.12.0");
    }

    #[test]
    fn query_results_include_alias_resolution_flag() {
        let versions = vec![remote("v20.11.0", None), remote("v22.1.0", Some("Jod"))];
        let search = search_available_versions_with_index(
            &versions,
            None,
            "stable",
            20,
            &HashSet::new(),
            &HashSet::new(),
            None,
        );

        assert!(search.alias_resolved);
        assert_eq!(search.versions.len(), 1);
        assert_eq!(search.versions[0].version.to_string(), "v22.1.0");
    }

    #[test]
    fn query_filters_apply_installed_and_eol_constraints() {
        let versions = vec![
            remote("v22.1.0", Some("Jod")),
            remote("v20.11.0", Some("Iron")),
        ];
        let installed = HashSet::from([versi_backend::NodeVersion::new(20, 11, 0)]);
        let filters = HashSet::from([SearchFilter::Installed, SearchFilter::Eol]);
        let schedule = schedule_with_eol_major(20);

        let search = search_available_versions_with_index(
            &versions,
            None,
            "v",
            20,
            &filters,
            &installed,
            Some(&schedule),
        );
        assert_eq!(search.versions.len(), 1);
        assert_eq!(search.versions[0].version.to_string(), "v20.11.0");
        assert!(!search.alias_resolved);
    }

    #[test]
    fn limit_is_applied_after_filters_to_fill_result_window() {
        let versions = vec![
            remote("v22.3.0", None),
            remote("v22.2.0", None),
            remote("v22.1.0", None),
        ];
        let installed = HashSet::from([versi_backend::NodeVersion::new(22, 2, 0)]);
        let filters = HashSet::from([SearchFilter::Installed]);

        let search = search_available_versions_with_index(
            &versions, None, "v22", 1, &filters, &installed, None,
        );

        assert_eq!(search.versions.len(), 1);
        assert_eq!(search.versions[0].version.to_string(), "v22.2.0");
    }

    #[test]
    fn alias_resolution_respects_active_filters() {
        let versions = vec![remote("v22.1.0", Some("Jod")), remote("v20.11.0", None)];
        let installed = HashSet::from([versi_backend::NodeVersion::new(22, 1, 0)]);
        let filters = HashSet::from([SearchFilter::NotInstalled]);

        let search = search_available_versions_with_index(
            &versions, None, "stable", 10, &filters, &installed, None,
        );

        assert!(search.alias_resolved);
        assert!(search.versions.is_empty());
    }

    #[test]
    fn indexed_search_matches_unindexed_behavior() {
        let versions = vec![
            remote("v22.3.0", Some("Jod")),
            remote("v22.2.0", Some("Jod")),
            remote("v20.11.0", Some("Iron")),
            remote("v20.10.0", Some("Iron")),
        ];
        let search_index = RemoteVersionSearchIndex::from_versions(&versions);
        let installed = HashSet::from([versi_backend::NodeVersion::new(22, 2, 0)]);
        let filters = HashSet::from([SearchFilter::Installed]);
        let schedule = schedule_with_eol_major(20);

        let unindexed = search_available_versions_with_index(
            &versions,
            None,
            "v22",
            10,
            &filters,
            &installed,
            Some(&schedule),
        );
        let indexed = search_available_versions_with_index(
            &versions,
            Some(&search_index),
            "v22",
            10,
            &filters,
            &installed,
            Some(&schedule),
        );

        let unindexed_versions: Vec<String> = unindexed
            .versions
            .iter()
            .map(|version| version.version.to_string())
            .collect();
        let indexed_versions: Vec<String> = indexed
            .versions
            .iter()
            .map(|version| version.version.to_string())
            .collect();

        assert_eq!(indexed.alias_resolved, unindexed.alias_resolved);
        assert_eq!(indexed_versions, unindexed_versions);
    }

    #[test]
    fn indexed_search_falls_back_when_index_is_stale() {
        let versions = vec![remote("v22.3.0", None), remote("v22.2.0", None)];
        let stale_index = RemoteVersionSearchIndex::from_versions(&versions[..1]);

        let result = search_available_versions_with_index(
            &versions,
            Some(&stale_index),
            "v22",
            10,
            &HashSet::new(),
            &HashSet::new(),
            None,
        );

        assert_eq!(result.versions.len(), 2);
        assert_eq!(result.versions[0].version.to_string(), "v22.3.0");
        assert_eq!(result.versions[1].version.to_string(), "v22.2.0");
    }

    #[test]
    #[ignore = "performance baseline; run manually"]
    fn perf_search_available_versions_large_dataset() {
        let mut versions = Vec::new();
        for major in 18_u32..=28 {
            for minor in 0_u32..40 {
                for patch in 0_u32..3 {
                    versions.push(versi_backend::RemoteVersion {
                        version: versi_backend::NodeVersion::new(major, minor, patch),
                        lts_codename: if major % 2 == 0 {
                            Some(format!("LTS-{major}"))
                        } else {
                            None
                        },
                        is_latest: patch == 2,
                    });
                }
            }
        }
        let installed = HashSet::from([
            versi_backend::NodeVersion::new(20, 39, 2),
            versi_backend::NodeVersion::new(22, 39, 2),
            versi_backend::NodeVersion::new(24, 39, 2),
        ]);
        let filters = HashSet::from([SearchFilter::Installed, SearchFilter::Active]);
        let schedule = schedule_with_eol_major(20);
        let search_index = RemoteVersionSearchIndex::from_versions(&versions);

        let started = Instant::now();
        for _ in 0..200 {
            let result = search_available_versions_with_index(
                &versions,
                Some(&search_index),
                "v2",
                30,
                &filters,
                &installed,
                Some(&schedule),
            );
            std::hint::black_box(result.versions.len());
        }
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_secs(2),
            "search baseline exceeded: {elapsed:?}"
        );
    }
}
