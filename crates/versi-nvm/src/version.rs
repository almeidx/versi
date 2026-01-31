use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion};

pub fn parse_unix_installed(output: &str) -> Vec<InstalledVersion> {
    let mut default_version: Option<NodeVersion> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("default")
            && let Some(arrow_pos) = trimmed.find("-> ")
        {
            let resolved = if let Some(paren_arrow) = trimmed.find("(-> ") {
                let after = &trimmed[paren_arrow + 4..];
                after.trim_end_matches(')')
            } else {
                &trimmed[arrow_pos + 3..]
            };
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

        if let Ok(version) = version_str.parse::<NodeVersion>() {
            let is_default = default_version.as_ref() == Some(&version);
            versions.push(InstalledVersion {
                version,
                is_default,
                lts_codename: None,
                install_date: None,
                disk_size: None,
            });
        }
    }

    versions
}

pub fn parse_windows_installed(output: &str) -> Vec<InstalledVersion> {
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

        if let Ok(version) = version_str.parse::<NodeVersion>() {
            versions.push(InstalledVersion {
                version,
                is_default: is_default || is_current,
                lts_codename: None,
                install_date: None,
                disk_size: None,
            });
        }
    }

    versions
}

pub fn parse_unix_remote(output: &str) -> Vec<RemoteVersion> {
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

        let lts_codename = if let Some(start) = rest.find("LTS: ") {
            let after_lts = &rest[start + 5..];
            after_lts.split(')').next().map(|s| s.to_string())
        } else {
            None
        };

        let is_latest = rest.contains("Latest LTS");

        if let Ok(version) = version_str.parse::<NodeVersion>() {
            versions.push(RemoteVersion {
                version,
                lts_codename,
                is_latest,
            });
        }
    }

    versions
}

pub fn parse_windows_remote(output: &str) -> Vec<RemoteVersion> {
    let mut versions = Vec::new();
    let mut in_table = false;

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.contains("CURRENT") || trimmed.contains("LTS") || trimmed.contains("OLD") {
            in_table = true;
            continue;
        }

        if !in_table || trimmed.is_empty() {
            continue;
        }

        let columns: Vec<&str> = trimmed.split_whitespace().collect();
        if columns.is_empty() {
            continue;
        }

        for col in &columns {
            let version_str = col.trim_start_matches('v');
            if let Ok(version) = version_str.parse::<NodeVersion>() {
                versions.push(RemoteVersion {
                    version,
                    lts_codename: None,
                    is_latest: false,
                });
            }
        }
    }

    versions
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.next() == Some('[') {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn clean_output(output: &str) -> String {
    strip_ansi(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unix_installed_basic() {
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
    fn test_parse_unix_installed_empty() {
        let output = "";
        let versions = parse_unix_installed(output);
        assert!(versions.is_empty());
    }

    #[test]
    fn test_parse_unix_installed_skips_aliases() {
        let output = "->     v20.11.0\n       v18.19.1\ndefault -> 20 (-> v20.11.0)\nnode -> stable (-> v20.11.0) (default)\nstable -> 20.11 (-> v20.11.0)\nlts/* -> lts/iron (-> v20.11.0)\nlts/iron -> v20.11.0\n";
        let versions = parse_unix_installed(output);
        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn test_parse_windows_installed_basic() {
        let output = "  * 20.11.0 (Currently using 64-bit executable)\n    18.19.1\n";
        let versions = parse_windows_installed(output);

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version.major, 20);
        assert!(versions[0].is_default);
        assert_eq!(versions[1].version.major, 18);
        assert!(!versions[1].is_default);
    }

    #[test]
    fn test_parse_unix_remote_basic() {
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
    fn test_parse_unix_remote_with_lts() {
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
    fn test_clean_output_strips_ansi() {
        let input = "\x1b[32m->     v20.11.0\x1b[0m";
        let cleaned = clean_output(input);
        assert_eq!(cleaned, "->     v20.11.0");
    }

    #[test]
    fn test_clean_output_no_ansi() {
        let input = "v20.11.0";
        let cleaned = clean_output(input);
        assert_eq!(cleaned, "v20.11.0");
    }

    #[test]
    fn test_parse_windows_remote_table() {
        let output = "|   CURRENT    |     LTS      |  OLD STABLE  | OLD UNSTABLE |\n|--------------|--------------|--------------|              |\n|    21.6.1    |   20.11.1    |   18.19.1    |              |\n|    21.6.0    |   20.11.0    |   18.19.0    |              |\n";
        let versions = parse_windows_remote(output);

        assert!(!versions.is_empty());
        let majors: Vec<u32> = versions.iter().map(|v| v.version.major).collect();
        assert!(majors.contains(&21));
        assert!(majors.contains(&20));
        assert!(majors.contains(&18));
    }
}
