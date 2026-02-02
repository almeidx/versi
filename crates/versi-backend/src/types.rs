use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl NodeVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn major_group(&self) -> u32 {
        self.major
    }
}

impl Ord for NodeVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl PartialOrd for NodeVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for NodeVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone)]
pub struct VersionParseError(pub String);

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse version: {}", self.0)
    }
}

impl std::error::Error for VersionParseError {}

impl FromStr for NodeVersion {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().strip_prefix('v').unwrap_or(s.trim());
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() < 3 {
            return Err(VersionParseError(format!(
                "Expected X.Y.Z format, got: {}",
                s
            )));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| VersionParseError(format!("Invalid major version: {}", parts[0])))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| VersionParseError(format!("Invalid minor version: {}", parts[1])))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| VersionParseError(format!("Invalid patch version: {}", parts[2])))?;

        Ok(NodeVersion::new(major, minor, patch))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub version: NodeVersion,
    pub is_default: bool,
    pub lts_codename: Option<String>,
    pub install_date: Option<chrono::DateTime<chrono::Utc>>,
    pub disk_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteVersion {
    pub version: NodeVersion,
    pub lts_codename: Option<String>,
    pub is_latest: bool,
}

#[derive(Debug, Clone)]
pub struct VersionGroup {
    pub major: u32,
    pub versions: Vec<InstalledVersion>,
    pub is_expanded: bool,
}

impl VersionGroup {
    pub fn from_versions(versions: Vec<InstalledVersion>) -> Vec<Self> {
        use std::collections::BTreeMap;

        let mut groups: BTreeMap<u32, Vec<InstalledVersion>> = BTreeMap::new();

        for version in versions {
            groups
                .entry(version.version.major)
                .or_default()
                .push(version);
        }

        groups
            .into_iter()
            .rev()
            .map(|(major, mut versions)| {
                versions.sort_by(|a, b| b.version.cmp(&a.version));
                VersionGroup {
                    major,
                    versions,
                    is_expanded: true,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_with_v_prefix() {
        let v: NodeVersion = "v20.11.0".parse().unwrap();
        assert_eq!(v.major, 20);
        assert_eq!(v.minor, 11);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_parse_version_without_v_prefix() {
        let v: NodeVersion = "20.11.0".parse().unwrap();
        assert_eq!(v.major, 20);
        assert_eq!(v.minor, 11);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_parse_version_with_whitespace() {
        let v: NodeVersion = "  v20.11.0  ".parse().unwrap();
        assert_eq!(v.major, 20);
    }

    #[test]
    fn test_parse_version_invalid_format() {
        let result: Result<NodeVersion, _> = "v20.11".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_version_invalid_major() {
        let result: Result<NodeVersion, _> = "vXX.11.0".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_version_display() {
        let v = NodeVersion::new(20, 11, 0);
        assert_eq!(v.to_string(), "v20.11.0");
    }

    #[test]
    fn test_version_ordering_by_major() {
        let v1: NodeVersion = "v18.0.0".parse().unwrap();
        let v2: NodeVersion = "v20.0.0".parse().unwrap();
        assert!(v2 > v1);
    }

    #[test]
    fn test_version_ordering_by_minor() {
        let v1: NodeVersion = "v20.10.0".parse().unwrap();
        let v2: NodeVersion = "v20.11.0".parse().unwrap();
        assert!(v2 > v1);
    }

    #[test]
    fn test_version_ordering_by_patch() {
        let v1: NodeVersion = "v20.11.0".parse().unwrap();
        let v2: NodeVersion = "v20.11.1".parse().unwrap();
        assert!(v2 > v1);
    }

    #[test]
    fn test_version_equality() {
        let v1: NodeVersion = "v20.11.0".parse().unwrap();
        let v2: NodeVersion = "v20.11.0".parse().unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_version_major_group() {
        let v = NodeVersion::new(20, 11, 0);
        assert_eq!(v.major_group(), 20);
    }

    #[test]
    fn test_version_group_from_versions() {
        let versions = vec![
            InstalledVersion {
                version: NodeVersion::new(20, 11, 0),
                is_default: true,
                lts_codename: None,
                install_date: None,
                disk_size: None,
            },
            InstalledVersion {
                version: NodeVersion::new(20, 10, 0),
                is_default: false,
                lts_codename: None,
                install_date: None,
                disk_size: None,
            },
            InstalledVersion {
                version: NodeVersion::new(18, 19, 0),
                is_default: false,
                lts_codename: None,
                install_date: None,
                disk_size: None,
            },
        ];

        let groups = VersionGroup::from_versions(versions);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].major, 20);
        assert_eq!(groups[1].major, 18);
        assert_eq!(groups[0].versions.len(), 2);
        assert_eq!(groups[1].versions.len(), 1);
    }

    #[test]
    fn test_version_group_sorted_descending() {
        let versions = vec![
            InstalledVersion {
                version: NodeVersion::new(20, 10, 0),
                is_default: false,
                lts_codename: None,
                install_date: None,
                disk_size: None,
            },
            InstalledVersion {
                version: NodeVersion::new(20, 11, 0),
                is_default: false,
                lts_codename: None,
                install_date: None,
                disk_size: None,
            },
        ];

        let groups = VersionGroup::from_versions(versions);

        assert_eq!(groups[0].versions[0].version.minor, 11);
        assert_eq!(groups[0].versions[1].version.minor, 10);
    }

    #[test]
    fn test_version_group_empty() {
        let versions: Vec<InstalledVersion> = vec![];
        let groups = VersionGroup::from_versions(versions);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_version_group_is_expanded_default() {
        let versions = vec![InstalledVersion {
            version: NodeVersion::new(20, 11, 0),
            is_default: false,
            lts_codename: None,
            install_date: None,
            disk_size: None,
        }];

        let groups = VersionGroup::from_versions(versions);
        assert!(groups[0].is_expanded);
    }
}
