use std::collections::HashMap;

use versi_backend::RemoteVersion;

pub(super) fn resolve_alias<'a>(
    versions: &'a [RemoteVersion],
    query: &str,
) -> Option<&'a RemoteVersion> {
    let query_lower = query.to_lowercase();

    match query_lower.as_str() {
        "latest" | "stable" | "current" => versions.iter().max_by_key(|v| &v.version),
        "lts/*" => versions
            .iter()
            .filter(|v| v.lts_codename.is_some())
            .max_by_key(|v| &v.version),
        q if q.starts_with("lts/") => {
            let codename = &q[4..];
            versions
                .iter()
                .filter(|v| {
                    v.lts_codename
                        .as_ref()
                        .is_some_and(|c| c.to_lowercase() == codename)
                })
                .max_by_key(|v| &v.version)
        }
        _ => None,
    }
}

pub(super) fn filter_available_versions<'a>(
    versions: &'a [RemoteVersion],
    query: &str,
) -> Vec<&'a RemoteVersion> {
    let query_lower = query.to_lowercase();

    if let Some(resolved) = resolve_alias(versions, query) {
        return vec![resolved];
    }

    if query_lower == "lts" {
        let mut filtered: Vec<&RemoteVersion> = versions
            .iter()
            .filter(|v| v.lts_codename.is_some())
            .collect();
        filtered.sort_by(|a, b| b.version.cmp(&a.version));

        let mut latest_by_major: HashMap<u32, &RemoteVersion> = HashMap::new();
        for v in &filtered {
            latest_by_major
                .entry(v.version.major)
                .and_modify(|existing| {
                    if v.version > existing.version {
                        *existing = v;
                    }
                })
                .or_insert(v);
        }

        let mut result: Vec<&RemoteVersion> = latest_by_major.into_values().collect();
        result.sort_by(|a, b| b.version.cmp(&a.version));
        result.truncate(20);
        return result;
    }

    let mut filtered: Vec<&RemoteVersion> = versions
        .iter()
        .filter(|v| {
            let version_str = v.version.to_string();

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
