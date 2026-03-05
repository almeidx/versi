use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::http::response_snippet;

const SCHEDULE_URL: &str = "https://raw.githubusercontent.com/nodejs/Release/main/schedule.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionSchedule {
    pub start: String,
    #[serde(default)]
    pub lts: Option<String>,
    #[serde(default)]
    pub maintenance: Option<String>,
    pub end: String,
    #[serde(default)]
    pub codename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseSchedule {
    pub versions: HashMap<u32, VersionSchedule>,
}

#[derive(Debug, Error)]
pub enum ScheduleError {
    #[error("failed to fetch release schedule: {0}")]
    Request(#[source] reqwest::Error),
    #[error("failed to fetch release schedule: HTTP {status}{body_snippet}")]
    HttpStatus {
        status: reqwest::StatusCode,
        body_snippet: String,
    },
    #[error("failed to parse release schedule: {0}")]
    Parse(#[source] reqwest::Error),
}

impl ReleaseSchedule {
    #[must_use]
    pub fn is_active(&self, major: u32) -> bool {
        let Some(schedule) = self.versions.get(&major) else {
            return major >= 18;
        };

        let Ok(end_date) = NaiveDate::parse_from_str(&schedule.end, "%Y-%m-%d") else {
            return true;
        };

        let today = chrono::Utc::now().date_naive();
        end_date > today
    }

    #[must_use]
    pub fn is_lts(&self, major: u32) -> bool {
        self.versions
            .get(&major)
            .is_some_and(|s| s.lts.is_some() || s.codename.is_some())
    }

    #[must_use]
    pub fn codename(&self, major: u32) -> Option<&str> {
        self.versions
            .get(&major)
            .and_then(|s| s.codename.as_deref())
    }
}

/// Fetch and parse the Node.js release schedule.
///
/// # Errors
/// Returns an error when the schedule cannot be downloaded or deserialized.
pub async fn fetch_release_schedule(
    client: &reqwest::Client,
) -> Result<ReleaseSchedule, ScheduleError> {
    let response = client
        .get(SCHEDULE_URL)
        .header("User-Agent", crate::http::USER_AGENT)
        .send()
        .await
        .map_err(ScheduleError::Request)?;

    if !response.status().is_success() {
        let status = response.status();
        let body_snippet = response
            .text()
            .await
            .ok()
            .map(|body| response_snippet(&body, 160))
            .unwrap_or_default();
        return Err(ScheduleError::HttpStatus {
            status,
            body_snippet,
        });
    }

    let raw: HashMap<String, VersionSchedule> =
        response.json().await.map_err(ScheduleError::Parse)?;

    let versions: HashMap<u32, VersionSchedule> = raw
        .into_iter()
        .filter_map(|(key, value)| {
            let major = key.trim_start_matches('v').parse().ok()?;
            Some((major, value))
        })
        .collect();

    Ok(ReleaseSchedule { versions })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_schedule() -> ReleaseSchedule {
        let mut versions = HashMap::new();

        versions.insert(
            20,
            VersionSchedule {
                start: "2023-04-18".to_string(),
                lts: Some("2023-10-24".to_string()),
                maintenance: Some("2024-10-22".to_string()),
                end: "2026-04-30".to_string(),
                codename: Some("Iron".to_string()),
            },
        );

        versions.insert(
            18,
            VersionSchedule {
                start: "2022-04-19".to_string(),
                lts: Some("2022-10-25".to_string()),
                maintenance: Some("2023-10-18".to_string()),
                end: "2025-04-30".to_string(),
                codename: Some("Hydrogen".to_string()),
            },
        );

        versions.insert(
            16,
            VersionSchedule {
                start: "2021-04-20".to_string(),
                lts: Some("2021-10-26".to_string()),
                maintenance: Some("2022-10-18".to_string()),
                end: "2023-09-11".to_string(),
                codename: Some("Gallium".to_string()),
            },
        );

        versions.insert(
            23,
            VersionSchedule {
                start: "2024-04-23".to_string(),
                lts: None,
                maintenance: None,
                end: "2025-06-01".to_string(),
                codename: None,
            },
        );

        ReleaseSchedule { versions }
    }

    #[test]
    fn test_is_lts_with_codename() {
        let schedule = create_test_schedule();
        assert!(schedule.is_lts(20));
        assert!(schedule.is_lts(18));
    }

    #[test]
    fn test_is_lts_without_codename() {
        let schedule = create_test_schedule();
        assert!(!schedule.is_lts(23));
    }

    #[test]
    fn test_is_lts_unknown_version() {
        let schedule = create_test_schedule();
        assert!(!schedule.is_lts(99));
    }

    #[test]
    fn test_codename() {
        let schedule = create_test_schedule();
        assert_eq!(schedule.codename(20), Some("Iron"));
        assert_eq!(schedule.codename(18), Some("Hydrogen"));
        assert_eq!(schedule.codename(23), None);
    }

    #[test]
    fn test_codename_unknown_version() {
        let schedule = create_test_schedule();
        assert_eq!(schedule.codename(99), None);
    }

    #[test]
    fn test_is_active_unknown_version_high() {
        let schedule = create_test_schedule();
        assert!(schedule.is_active(99));
    }

    #[test]
    fn test_is_active_unknown_version_low() {
        let schedule = create_test_schedule();
        assert!(!schedule.is_active(10));
    }

    #[test]
    fn test_is_active_eol_version() {
        let schedule = create_test_schedule();
        assert!(!schedule.is_active(16));
    }
}
