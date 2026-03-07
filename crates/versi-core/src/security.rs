use std::collections::HashMap;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::http::response_snippet;

const SECURITY_INDEX_URL: &str =
    "https://raw.githubusercontent.com/nodejs/security-wg/main/vuln/core/index.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAdvisory {
    #[serde(default)]
    pub cve: Vec<String>,
    #[serde(default)]
    pub vulnerable: String,
    #[serde(default)]
    pub patched: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default, rename = "ref")]
    pub reference: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub overview: String,
    #[serde(default, rename = "affectedEnvironments")]
    pub affected_environments: Vec<String>,
}

impl SecurityAdvisory {
    #[must_use]
    pub fn affects_version_on_platform(&self, version: &str, platform: &str) -> bool {
        if !self.affected_environment_matches(platform) {
            return false;
        }

        let Some(version) = parse_node_semver(version) else {
            return false;
        };

        let vulnerable = matches_requirement_expression(&self.vulnerable, &version);
        if !vulnerable {
            return false;
        }

        let patched = matches_requirement_expression(&self.patched, &version);
        !patched
    }

    #[must_use]
    pub fn prepare(&self) -> PreparedAdvisory<'_> {
        PreparedAdvisory {
            vulnerable: parse_requirement_expression(&self.vulnerable),
            patched: parse_requirement_expression(&self.patched),
            advisory: self,
        }
    }

    fn affected_environment_matches(&self, platform: &str) -> bool {
        if self.affected_environments.is_empty() {
            return true;
        }

        let platform = platform.to_ascii_lowercase();
        self.affected_environments.iter().any(|entry| {
            let entry = entry.to_ascii_lowercase();
            entry == "all" || entry == platform
        })
    }
}

#[derive(Debug, Error)]
pub enum SecurityAdvisoryError {
    #[error("failed to fetch security advisory index: {0}")]
    Request(#[source] reqwest::Error),
    #[error("failed to fetch security advisory index: HTTP {status}{body_snippet}")]
    HttpStatus {
        status: reqwest::StatusCode,
        body_snippet: String,
    },
    #[error("failed to parse security advisory index: {0}")]
    Parse(#[source] reqwest::Error),
}

/// Fetch Node.js core security advisories.
///
/// # Errors
/// Returns an error when the advisory index cannot be downloaded or parsed.
pub async fn fetch_security_advisories(
    client: &reqwest::Client,
) -> Result<HashMap<String, SecurityAdvisory>, SecurityAdvisoryError> {
    let response = client
        .get(SECURITY_INDEX_URL)
        .header("User-Agent", crate::http::USER_AGENT)
        .send()
        .await
        .map_err(SecurityAdvisoryError::Request)?;

    if !response.status().is_success() {
        let status = response.status();
        let body_snippet = response
            .text()
            .await
            .ok()
            .map(|body| response_snippet(&body, 160))
            .unwrap_or_default();
        return Err(SecurityAdvisoryError::HttpStatus {
            status,
            body_snippet,
        });
    }

    response.json().await.map_err(SecurityAdvisoryError::Parse)
}

fn parse_node_semver(input: &str) -> Option<Version> {
    let normalized = input.trim().strip_prefix('v').unwrap_or(input.trim());
    Version::parse(normalized).ok()
}

fn parse_requirement_expression(requirement: &str) -> Vec<VersionReq> {
    requirement
        .split("||")
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .filter_map(|clause| VersionReq::parse(clause).ok())
        .collect()
}

fn matches_requirement_expression(requirement: &str, version: &Version) -> bool {
    parse_requirement_expression(requirement)
        .iter()
        .any(|req| req.matches(version))
}

pub struct PreparedAdvisory<'a> {
    vulnerable: Vec<VersionReq>,
    patched: Vec<VersionReq>,
    advisory: &'a SecurityAdvisory,
}

impl PreparedAdvisory<'_> {
    #[must_use]
    pub fn affects_version_on_platform(&self, version: &str, platform: &str) -> bool {
        if !self.advisory.affected_environment_matches(platform) {
            return false;
        }

        let Some(version) = parse_node_semver(version) else {
            return false;
        };

        let vulnerable = self.vulnerable.iter().any(|req| req.matches(&version));
        if !vulnerable {
            return false;
        }

        let patched = self.patched.iter().any(|req| req.matches(&version));
        !patched
    }
}

#[cfg(test)]
mod tests {
    use super::SecurityAdvisory;

    fn advisory(
        vulnerable: &str,
        patched: &str,
        affected_environments: &[&str],
    ) -> SecurityAdvisory {
        SecurityAdvisory {
            cve: vec!["CVE-2026-0001".to_string()],
            vulnerable: vulnerable.to_string(),
            patched: patched.to_string(),
            severity: "high".to_string(),
            reference: "https://nodejs.org/en/blog/vulnerability/example".to_string(),
            description: "example".to_string(),
            overview: "example overview".to_string(),
            affected_environments: affected_environments
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
        }
    }

    #[test]
    fn affects_version_on_platform_supports_or_clauses() {
        let advisory = advisory("20.x || 22.x", "^20.20.0 || ^22.22.0", &["all"]);

        assert!(advisory.affects_version_on_platform("v20.19.4", "darwin"));
        assert!(advisory.affects_version_on_platform("22.21.1", "linux"));
        assert!(!advisory.affects_version_on_platform("24.1.0", "linux"));
    }

    #[test]
    fn affects_version_on_platform_handles_wildcards_and_comparators() {
        let advisory = advisory("<= 10 || <6.2.0", ">= 10.9.0 || ^6.2.1", &["all"]);

        assert!(advisory.affects_version_on_platform("v10.8.0", "linux"));
        assert!(advisory.affects_version_on_platform("v6.1.0", "linux"));
        assert!(!advisory.affects_version_on_platform("v10.9.0", "linux"));
        assert!(!advisory.affects_version_on_platform("v6.2.1", "linux"));
    }

    #[test]
    fn affects_version_on_platform_excludes_patched_versions() {
        let advisory = advisory("24.x", "^24.13.0", &["all"]);

        assert!(advisory.affects_version_on_platform("v24.12.0", "darwin"));
        assert!(!advisory.affects_version_on_platform("v24.13.0", "darwin"));
    }

    #[test]
    fn affects_version_on_platform_filters_platforms() {
        let advisory = advisory("22.x", "^22.22.0", &["linux", "win32"]);

        assert!(advisory.affects_version_on_platform("v22.21.1", "linux"));
        assert!(advisory.affects_version_on_platform("v22.21.1", "win32"));
        assert!(!advisory.affects_version_on_platform("v22.21.1", "darwin"));
    }

    #[test]
    fn affects_version_on_platform_returns_false_for_unparseable_versions() {
        let advisory = advisory("22.x", "^22.22.0", &["all"]);

        assert!(!advisory.affects_version_on_platform("lts/*", "linux"));
    }
}
