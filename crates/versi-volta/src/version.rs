use serde::Deserialize;

use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeLine {
    version: NodeVersion,
    is_default: bool,
}

fn parse_runtime_line(line: &str) -> Option<RuntimeLine> {
    let line = line.trim();
    if !line.starts_with("runtime ") {
        return None;
    }

    let tool = line.split_whitespace().nth(1)?;
    let version = tool.strip_prefix("node@")?.parse().ok()?;
    let is_default = line.contains("(default)");

    Some(RuntimeLine {
        version,
        is_default,
    })
}

#[must_use]
pub(crate) fn parse_installed_versions(output: &str) -> Vec<InstalledVersion> {
    let mut versions: Vec<InstalledVersion> = Vec::new();

    for line in output.lines() {
        let Some(runtime) = parse_runtime_line(line) else {
            continue;
        };

        if let Some(existing) = versions.iter_mut().find(|v| v.version == runtime.version) {
            existing.is_default |= runtime.is_default;
            continue;
        }

        versions.push(InstalledVersion {
            version: runtime.version,
            is_default: runtime.is_default,
            lts_codename: None,
            disk_size: None,
        });
    }

    versions.sort_by(|a, b| b.version.cmp(&a.version));
    versions
}

#[must_use]
pub(crate) fn parse_first_runtime_version(output: &str) -> Option<NodeVersion> {
    output
        .lines()
        .find_map(parse_runtime_line)
        .map(|runtime| runtime.version)
}

#[derive(Debug, Deserialize)]
struct NodeIndexEntry {
    version: String,
    #[serde(default)]
    lts: Option<LtsValue>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LtsValue {
    Enabled(bool),
    Codename(String),
}

impl LtsValue {
    fn codename(self) -> Option<String> {
        match self {
            Self::Codename(name) if !name.is_empty() => Some(name),
            Self::Enabled(_enabled) => None,
            Self::Codename(_) => None,
        }
    }
}

/// Parses nodejs.org `index.json` entries into sorted remote versions.
///
/// # Errors
///
/// Returns an error if the response body is not valid JSON for the expected schema.
pub(crate) fn parse_node_index_remote_versions(
    body: &str,
) -> Result<Vec<RemoteVersion>, serde_json::Error> {
    let entries: Vec<NodeIndexEntry> = serde_json::from_str(body)?;

    let mut versions = entries
        .into_iter()
        .filter_map(|entry| {
            let version = entry.version.parse().ok()?;
            Some(RemoteVersion {
                version,
                lts_codename: entry.lts.and_then(LtsValue::codename),
                is_latest: false,
            })
        })
        .collect::<Vec<_>>();

    versions.sort_by(|a, b| b.version.cmp(&a.version));
    if let Some(first) = versions.first_mut() {
        first.is_latest = true;
    }

    Ok(versions)
}

#[cfg(test)]
mod tests {
    use super::{
        parse_first_runtime_version, parse_installed_versions, parse_node_index_remote_versions,
    };

    #[test]
    fn parse_installed_versions_extracts_runtime_and_default_marker() {
        let output = "runtime node@20.11.0 (default)\nruntime node@22.1.0\n";
        let versions = parse_installed_versions(output);

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version.to_string(), "v22.1.0");
        assert_eq!(versions[1].version.to_string(), "v20.11.0");
        assert!(versions[1].is_default);
    }

    #[test]
    fn parse_installed_versions_deduplicates_and_preserves_default() {
        let output = "runtime node@20.11.0\nruntime node@20.11.0 (default)\n";
        let versions = parse_installed_versions(output);

        assert_eq!(versions.len(), 1);
        assert!(versions[0].is_default);
    }

    #[test]
    fn parse_first_runtime_version_returns_none_for_empty_output() {
        assert!(parse_first_runtime_version("").is_none());
        assert!(parse_first_runtime_version("package yarn@1.22.0").is_none());
    }

    #[test]
    fn parse_first_runtime_version_returns_first_runtime() {
        let output =
            "runtime node@22.2.0 (current @ /tmp/project)\nruntime node@20.11.0 (default)\n";

        let parsed = parse_first_runtime_version(output).expect("runtime should parse");

        assert_eq!(parsed.to_string(), "v22.2.0");
    }

    #[test]
    fn parse_node_index_remote_versions_extracts_lts_and_latest() {
        let body = r#"
[
  {"version":"v22.5.1","lts":"Jod"},
  {"version":"v23.1.0","lts":false},
  {"version":"v20.18.0","lts":"Iron"}
]
"#;
        let versions = parse_node_index_remote_versions(body).expect("index should parse");

        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version.to_string(), "v23.1.0");
        assert!(versions[0].is_latest);
        assert_eq!(versions[1].lts_codename.as_deref(), Some("Jod"));
        assert_eq!(versions[2].lts_codename.as_deref(), Some("Iron"));
    }
}
