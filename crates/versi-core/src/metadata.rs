use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::http::response_snippet;

const INDEX_URL: &str = "https://nodejs.org/dist/index.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMeta {
    pub date: String,
    pub security: bool,
    pub npm: Option<String>,
    pub v8: Option<String>,
    pub openssl: Option<String>,
}

#[derive(Deserialize)]
struct RawEntry {
    version: String,
    date: String,
    #[serde(default)]
    security: bool,
    #[serde(default)]
    npm: Option<String>,
    #[serde(default)]
    v8: Option<String>,
    #[serde(default)]
    openssl: Option<String>,
}

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("failed to fetch version metadata: {0}")]
    Request(#[source] reqwest::Error),
    #[error("failed to fetch version metadata: HTTP {status}{body_snippet}")]
    HttpStatus {
        status: reqwest::StatusCode,
        body_snippet: String,
    },
    #[error("failed to parse version metadata: {0}")]
    Parse(#[source] reqwest::Error),
}

fn map_entries(entries: Vec<RawEntry>) -> HashMap<String, VersionMeta> {
    entries
        .into_iter()
        .map(|entry| {
            (
                entry.version,
                VersionMeta {
                    date: entry.date,
                    security: entry.security,
                    npm: entry.npm,
                    v8: entry.v8,
                    openssl: entry.openssl,
                },
            )
        })
        .collect()
}

/// Fetch Node.js version metadata from `nodejs.org`.
///
/// # Errors
/// Returns an error when the remote metadata cannot be fetched or parsed.
pub async fn fetch_version_metadata(
    client: &reqwest::Client,
) -> Result<HashMap<String, VersionMeta>, MetadataError> {
    let response = client
        .get(INDEX_URL)
        .header("User-Agent", crate::http::USER_AGENT)
        .send()
        .await
        .map_err(MetadataError::Request)?;

    if !response.status().is_success() {
        let status = response.status();
        let body_snippet = response
            .text()
            .await
            .ok()
            .map(|body| response_snippet(&body, 160))
            .unwrap_or_default();
        return Err(MetadataError::HttpStatus {
            status,
            body_snippet,
        });
    }

    let entries: Vec<RawEntry> = response.json().await.map_err(MetadataError::Parse)?;

    Ok(map_entries(entries))
}

#[cfg(test)]
mod tests {
    use super::{RawEntry, map_entries};

    #[test]
    fn map_entries_preserves_expected_fields() {
        let entries = vec![
            RawEntry {
                version: "v24.0.0".to_string(),
                date: "2026-01-01".to_string(),
                security: true,
                npm: Some("11.0.0".to_string()),
                v8: Some("12.0".to_string()),
                openssl: Some("3.4.0".to_string()),
            },
            RawEntry {
                version: "v22.5.0".to_string(),
                date: "2025-12-15".to_string(),
                security: false,
                npm: None,
                v8: None,
                openssl: None,
            },
        ];

        let mapped = map_entries(entries);

        assert_eq!(mapped.len(), 2);
        let v24 = mapped
            .get("v24.0.0")
            .expect("v24 metadata should be present");
        assert_eq!(v24.date, "2026-01-01");
        assert!(v24.security);
        assert_eq!(v24.npm.as_deref(), Some("11.0.0"));
        assert_eq!(v24.v8.as_deref(), Some("12.0"));
        assert_eq!(v24.openssl.as_deref(), Some("3.4.0"));
        let v22 = mapped
            .get("v22.5.0")
            .expect("v22 metadata should be present");
        assert!(!v22.security);
        assert!(v22.npm.is_none());
    }

    #[test]
    fn map_entries_overwrites_duplicate_versions_with_last_value() {
        let mapped = map_entries(vec![
            RawEntry {
                version: "v22.0.0".to_string(),
                date: "2025-04-24".to_string(),
                security: false,
                npm: Some("10.0.0".to_string()),
                v8: None,
                openssl: None,
            },
            RawEntry {
                version: "v22.0.0".to_string(),
                date: "2025-05-01".to_string(),
                security: true,
                npm: Some("10.1.0".to_string()),
                v8: Some("11.2".to_string()),
                openssl: Some("3.0.0".to_string()),
            },
        ]);

        let entry = mapped
            .get("v22.0.0")
            .expect("deduplicated key should exist");
        assert_eq!(entry.date, "2025-05-01");
        assert!(entry.security);
        assert_eq!(entry.npm.as_deref(), Some("10.1.0"));
        assert_eq!(entry.v8.as_deref(), Some("11.2"));
    }
}
