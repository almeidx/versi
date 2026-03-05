use versi_backend::{InstalledVersion, RemoteVersion};

#[must_use]
pub(crate) fn parse_installed_versions(output: &str) -> Vec<InstalledVersion> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            if line == "system" || line == "* system" {
                return None;
            }

            let is_default = line.contains("default");

            let version_str = line.split_whitespace().find(|s| s.starts_with('v'))?;

            let version = version_str.parse().ok()?;

            Some(InstalledVersion {
                version,
                is_default,
                lts_codename: None,
                disk_size: None,
            })
        })
        .collect()
}

#[must_use]
pub(crate) fn parse_remote_versions(output: &str) -> Vec<RemoteVersion> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            let (version_part, rest) = line.split_once(' ').unwrap_or((line, ""));
            let version = version_part.trim().parse().ok()?;

            let lts_codename = rest
                .trim()
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .map(str::to_string);

            Some(RemoteVersion {
                version,
                lts_codename,
                is_latest: false,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_installed_versions_basic() {
        let output = "* v20.11.0 default\nv18.19.1\nv16.20.2";
        let versions = parse_installed_versions(output);
        assert_eq!(versions.len(), 3);
        assert!(versions[0].is_default);
        assert!(!versions[1].is_default);
        assert!(!versions[2].is_default);
    }

    #[test]
    fn parse_installed_versions_empty() {
        let output = "";
        let versions = parse_installed_versions(output);
        assert!(versions.is_empty());
    }

    #[test]
    fn parse_installed_versions_with_whitespace() {
        let output = "  v20.11.0  \n  v18.19.1  \n";
        let versions = parse_installed_versions(output);
        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn parse_installed_versions_skips_system() {
        let output = "system\n* system\nv20.11.0";
        let versions = parse_installed_versions(output);
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version.major, 20);
    }

    #[test]
    fn parse_installed_versions_default_marker() {
        let output = "v20.11.0 default";
        let versions = parse_installed_versions(output);
        assert_eq!(versions.len(), 1);
        assert!(versions[0].is_default);
    }

    #[test]
    fn parse_remote_versions_basic() {
        let output = "v22.0.0\nv21.7.3\nv20.18.0 (Iron)";
        let versions = parse_remote_versions(output);
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version.major, 22);
        assert!(versions[0].lts_codename.is_none());
        assert_eq!(versions[2].lts_codename, Some("Iron".to_string()));
    }

    #[test]
    fn parse_remote_versions_empty() {
        let output = "";
        let versions = parse_remote_versions(output);
        assert!(versions.is_empty());
    }

    #[test]
    fn parse_remote_versions_lts_codename() {
        let output = "v20.18.0 (Iron)\nv18.20.0 (Hydrogen)";
        let versions = parse_remote_versions(output);
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].lts_codename, Some("Iron".to_string()));
        assert_eq!(versions[1].lts_codename, Some("Hydrogen".to_string()));
    }

    #[test]
    fn parse_remote_versions_no_lts() {
        let output = "v23.0.0\nv22.5.0";
        let versions = parse_remote_versions(output);
        assert_eq!(versions.len(), 2);
        assert!(versions[0].lts_codename.is_none());
        assert!(versions[1].lts_codename.is_none());
    }
}
