use std::collections::HashMap;

use versi_backend::{NodeVersion, RemoteVersion};

pub(super) fn compute_latest_by_major(
    remote_versions: &[RemoteVersion],
) -> HashMap<u32, NodeVersion> {
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

pub(super) fn filter_available_versions<'a>(
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
