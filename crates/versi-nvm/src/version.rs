use std::collections::HashMap;

use log::debug;
use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion};

pub(crate) fn parse_unix_installed(output: &str) -> Vec<InstalledVersion> {
    let mut default_version: Option<NodeVersion> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("default")
            && let Some((_, after_arrow)) = trimmed.split_once("-> ")
        {
            let resolved = after_arrow
                .rsplit_once("(-> ")
                .map_or(after_arrow, |(_, inner)| inner.trim_end_matches(')'));
            let version_str = resolved
                .trim()
                .trim_start_matches('v')
                .split(|c: char| !c.is_ascii_digit() && c != '.')
                .next()
                .unwrap_or("");
            if let Ok(v) = version_str.parse::<NodeVersion>() {
                default_version = Some(v);
            }
        }
    }

    let mut versions = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("default")
            || trimmed.starts_with("node")
            || trimmed.starts_with("stable")
            || trimmed.starts_with("iojs")
            || trimmed.starts_with("lts/")
            || trimmed.starts_with("system")
        {
            continue;
        }

        let is_current = trimmed.starts_with("->");
        let version_part = if is_current {
            trimmed.trim_start_matches("->").trim()
        } else {
            trimmed
        };

        let version_str = version_part.trim_start_matches('v');
        let version_str = version_str.split_whitespace().next().unwrap_or("");

        if version_str.is_empty() {
            continue;
        }

        match version_str.parse::<NodeVersion>() {
            Ok(version) => {
                let is_default = default_version.as_ref() == Some(&version);
                versions.push(InstalledVersion {
                    version,
                    is_default,
                    lts_codename: None,
                    disk_size: None,
                });
            }
            Err(e) => debug!("Skipping unparseable installed version {version_str:?}: {e}"),
        }
    }

    versions
}

pub(crate) fn parse_windows_installed(output: &str) -> Vec<InstalledVersion> {
    let mut versions = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let is_current = trimmed.contains("Currently using");
        let is_default = trimmed.starts_with('*');

        let version_part = trimmed
            .trim_start_matches('*')
            .split_whitespace()
            .next()
            .unwrap_or("");

        let version_str = version_part.trim_start_matches('v');
        if version_str.is_empty() {
            continue;
        }

        match version_str.parse::<NodeVersion>() {
            Ok(version) => {
                versions.push(InstalledVersion {
                    version,
                    is_default: is_default || is_current,
                    lts_codename: None,
                    disk_size: None,
                });
            }
            Err(e) => debug!("Skipping unparseable installed version {version_str:?}: {e}"),
        }
    }

    versions
}

pub(crate) fn parse_unix_remote(output: &str) -> Vec<RemoteVersion> {
    let mut versions = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let version_part = trimmed.trim_start_matches('v');
        let (version_str, rest) = version_part
            .split_once(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or((version_part, ""));

        if version_str.is_empty() {
            continue;
        }

        let lts_codename = rest
            .split_once("LTS: ")
            .and_then(|(_, after)| after.split(')').next())
            .map(str::to_string);

        let is_latest = rest.contains("Latest LTS");

        match version_str.parse::<NodeVersion>() {
            Ok(version) => {
                versions.push(RemoteVersion {
                    version,
                    lts_codename,
                    is_latest,
                });
            }
            Err(e) => debug!("Skipping unparseable remote version {version_str:?}: {e}"),
        }
    }

    versions
}

pub(crate) fn parse_windows_remote(output: &str) -> Vec<RemoteVersion> {
    let mut versions: Vec<RemoteVersion> = Vec::new();
    let mut index_by_version: HashMap<NodeVersion, usize> = HashMap::new();
    let mut lts_column = None;

    let mut upsert = |version: NodeVersion, is_lts: bool| {
        if let Some(index) = index_by_version.get(&version).copied() {
            if is_lts {
                versions[index].lts_codename = Some("LTS".to_string());
            }
            return;
        }

        let index = versions.len();
        versions.push(RemoteVersion {
            version,
            lts_codename: is_lts.then(|| "LTS".to_string()),
            is_latest: false,
        });
        index_by_version.insert(version, index);
    };

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !trimmed.contains('|') {
            for token in trimmed.split_whitespace() {
                let version_str = token.trim_start_matches('v');
                let Ok(version) = version_str.parse::<NodeVersion>() else {
                    continue;
                };
                upsert(version, false);
            }
            continue;
        }

        let columns = trimmed
            .split('|')
            .map(str::trim)
            .filter(|column| !column.is_empty())
            .collect::<Vec<_>>();
        if columns.is_empty() {
            continue;
        }

        if columns
            .iter()
            .all(|column| column.chars().all(|ch| ch == '-'))
        {
            continue;
        }

        if let Some(position) = columns
            .iter()
            .position(|column| column.eq_ignore_ascii_case("LTS"))
        {
            lts_column = Some(position);
            continue;
        }

        for (index, column) in columns.iter().enumerate() {
            let version_str = column.trim_start_matches('v');
            let Ok(version) = version_str.parse::<NodeVersion>() else {
                continue;
            };
            upsert(version, lts_column == Some(index));
        }
    }

    versions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unix_installed_basic() {
        let output = "->     v20.11.0\n       v18.19.1\ndefault -> 20 (-> v20.11.0)\n";
        let versions = parse_unix_installed(output);

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version.major, 20);
        assert_eq!(versions[0].version.minor, 11);
        assert!(versions[0].is_default);
        assert_eq!(versions[1].version.major, 18);
        assert!(!versions[1].is_default);
    }

    #[test]
    fn parse_unix_installed_empty() {
        let output = "";
        let versions = parse_unix_installed(output);
        assert!(versions.is_empty());
    }

    #[test]
    fn parse_unix_installed_skips_aliases() {
        let output = "->     v20.11.0\n       v18.19.1\ndefault -> 20 (-> v20.11.0)\nnode -> stable (-> v20.11.0) (default)\nstable -> 20.11 (-> v20.11.0)\nlts/* -> lts/iron (-> v20.11.0)\nlts/iron -> v20.11.0\n";
        let versions = parse_unix_installed(output);
        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn parse_windows_installed_basic() {
        let output = "  * 20.11.0 (Currently using 64-bit executable)\n    18.19.1\n";
        let versions = parse_windows_installed(output);

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version.major, 20);
        assert!(versions[0].is_default);
        assert_eq!(versions[1].version.major, 18);
        assert!(!versions[1].is_default);
    }

    #[test]
    fn parse_unix_remote_basic() {
        let output = "        v20.10.0\n        v20.11.0   (Latest LTS: Iron)\n        v21.0.0\n";
        let versions = parse_unix_remote(output);

        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version.major, 20);
        assert!(versions[0].lts_codename.is_none());
        assert_eq!(versions[1].version.major, 20);
        assert_eq!(versions[1].lts_codename.as_deref(), Some("Iron"));
        assert!(versions[1].is_latest);
        assert_eq!(versions[2].version.major, 21);
    }

    #[test]
    fn parse_unix_remote_with_lts() {
        let output =
            "        v18.19.0   (LTS: Hydrogen)\n        v18.19.1   (Latest LTS: Hydrogen)\n";
        let versions = parse_unix_remote(output);

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].lts_codename.as_deref(), Some("Hydrogen"));
        assert!(!versions[0].is_latest);
        assert_eq!(versions[1].lts_codename.as_deref(), Some("Hydrogen"));
        assert!(versions[1].is_latest);
    }

    #[test]
    fn parse_windows_remote_table() {
        let output = "|   CURRENT    |     LTS      |  OLD STABLE  | OLD UNSTABLE |\n|--------------|--------------|--------------|              |\n|    21.6.1    |   20.11.1    |   18.19.1    |              |\n|    21.6.0    |   20.11.0    |   18.19.0    |              |\n";
        let versions = parse_windows_remote(output);

        assert!(!versions.is_empty());
        let majors: Vec<u32> = versions.iter().map(|v| v.version.major).collect();
        assert!(majors.contains(&21));
        assert!(majors.contains(&20));
        assert!(majors.contains(&18));
        assert!(versions.iter().any(
            |v| v.version.to_string() == "v20.11.1" && v.lts_codename.as_deref() == Some("LTS")
        ));
    }

    #[test]
    fn parse_windows_remote_deduplicates_versions() {
        let output =
            "| CURRENT | LTS |\n|---------|-----|\n| 21.6.1 | 20.11.1 |\n| 21.6.1 | 20.11.1 |\n";
        let versions = parse_windows_remote(output);

        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn parse_windows_remote_upgrades_duplicate_to_lts_when_seen_later() {
        let output = "21.6.1\n| CURRENT | LTS |\n|---------|-----|\n| 21.6.1 | 21.6.1 |\n";
        let versions = parse_windows_remote(output);

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version.to_string(), "v21.6.1");
        assert_eq!(versions[0].lts_codename.as_deref(), Some("LTS"));
    }
}
