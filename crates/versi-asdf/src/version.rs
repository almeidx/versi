use std::collections::HashSet;

use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion};

fn parse_version_token(token: &str) -> Option<NodeVersion> {
    let token = token.trim_matches(|ch: char| matches!(ch, '*' | '|' | '-' | '>'));
    if token.is_empty() {
        return None;
    }

    let mut normalized = String::with_capacity(token.len());
    for (idx, ch) in token.chars().enumerate() {
        if idx == 0 && ch == 'v' {
            normalized.push(ch);
            continue;
        }

        if ch.is_ascii_digit() || ch == '.' {
            normalized.push(ch);
        } else {
            return None;
        }
    }

    if normalized.matches('.').count() != 2 {
        return None;
    }

    normalized.parse().ok()
}

#[must_use]
pub fn parse_installed_versions(output: &str) -> Vec<InstalledVersion> {
    let mut versions = Vec::new();
    let mut seen = HashSet::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.contains("No versions installed") {
            continue;
        }

        let Some(version) = line
            .split_whitespace()
            .find_map(parse_version_token)
            .filter(|version| seen.insert(*version))
        else {
            continue;
        };

        versions.push(InstalledVersion {
            version,
            is_default: false,
            lts_codename: None,
            disk_size: None,
        });
    }

    versions
}

#[must_use]
pub fn parse_remote_versions(output: &str) -> Vec<RemoteVersion> {
    let mut versions = Vec::new();
    let mut seen = HashSet::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some(version) = line
            .split_whitespace()
            .find_map(parse_version_token)
            .filter(|version| seen.insert(*version))
        else {
            continue;
        };

        versions.push(RemoteVersion {
            version,
            lts_codename: None,
            is_latest: false,
        });
    }

    versions
}

#[must_use]
pub fn parse_current_version(output: &str) -> Option<NodeVersion> {
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let line_lower = line.to_ascii_lowercase();
        if line_lower.contains("no version") || line_lower.contains("not installed") {
            return None;
        }

        if let Some(version) = line.split_whitespace().find_map(parse_version_token) {
            return Some(version);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{parse_current_version, parse_installed_versions, parse_remote_versions};

    #[test]
    fn parse_installed_versions_supports_current_markers() {
        let output = "  18.20.8\n *20.11.1\n";

        let parsed = parse_installed_versions(output);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].version.to_string(), "v18.20.8");
        assert_eq!(parsed[1].version.to_string(), "v20.11.1");
    }

    #[test]
    fn parse_installed_versions_skips_non_version_lines() {
        let output = "No versions installed\n garbage \n";

        assert!(parse_installed_versions(output).is_empty());
    }

    #[test]
    fn parse_remote_versions_extracts_node_versions() {
        let output = "24.2.0\n24.1.1\nref:master\n";

        let parsed = parse_remote_versions(output);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].version.to_string(), "v24.2.0");
        assert_eq!(parsed[1].version.to_string(), "v24.1.1");
    }

    #[test]
    fn parse_remote_versions_skips_prerelease_tokens() {
        let output = "24.2.0-rc.1\n24.1.1\n";

        let parsed = parse_remote_versions(output);

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].version.to_string(), "v24.1.1");
    }

    #[test]
    fn parse_current_version_reads_tabular_output() {
        let output = "nodejs 20.11.1 /Users/test/.tool-versions true\n";

        let parsed = parse_current_version(output);

        assert_eq!(
            parsed
                .expect("version should parse from asdf current output")
                .to_string(),
            "v20.11.1"
        );
    }

    #[test]
    fn parse_current_version_handles_unset_version_message() {
        let output = "No version is set for command node\n";

        assert!(parse_current_version(output).is_none());
    }

    #[test]
    fn parse_current_version_handles_not_installed_message_with_capitalization() {
        let output = "nodejs 20.11.1 Not installed. Run \"asdf install nodejs 20.11.1\"\n";

        assert!(parse_current_version(output).is_none());
    }
}
