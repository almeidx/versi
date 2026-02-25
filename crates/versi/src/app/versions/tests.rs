use std::collections::HashMap;

use tokio_util::sync::CancellationToken;

use super::super::test_app_with_two_environments;
use super::*;
use crate::settings::AppUpdateBehavior;

fn remote(version: &str, is_latest: bool) -> versi_backend::RemoteVersion {
    versi_backend::RemoteVersion {
        version: version.parse().expect("test version should parse"),
        lts_codename: None,
        is_latest,
    }
}

fn sample_schedule() -> versi_core::ReleaseSchedule {
    serde_json::from_value(serde_json::json!({
        "versions": {
            "22": {
                "start": "2024-04-23",
                "lts": "2024-10-29",
                "maintenance": "2026-10-20",
                "end": "2027-04-30",
                "codename": "Jod"
            }
        }
    }))
    .expect("sample release schedule should deserialize")
}

fn sample_metadata() -> HashMap<String, versi_core::VersionMeta> {
    HashMap::from([(
        "v22.10.0".to_string(),
        versi_core::VersionMeta {
            date: "2026-01-01".to_string(),
            security: true,
            npm: Some("11.0.0".to_string()),
            v8: Some("12.0".to_string()),
            openssl: Some("3.4.0".to_string()),
        },
    )])
}

fn sample_security_advisories() -> HashMap<String, versi_core::SecurityAdvisory> {
    HashMap::from([(
        "163".to_string(),
        versi_core::SecurityAdvisory {
            cve: vec!["CVE-2026-21637".to_string()],
            vulnerable: "20.x || 22.x || 24.x || 25.x".to_string(),
            patched: "^20.20.0 || ^22.22.0 || ^24.13.0 || ^25.3.0".to_string(),
            severity: "medium".to_string(),
            reference: "https://nodejs.org/en/blog/vulnerability/december-2025-security-releases"
                .to_string(),
            description: "TLS callback issue".to_string(),
            overview: "overview".to_string(),
            affected_environments: vec!["all".to_string()],
        },
    )])
}

#[test]
fn remote_versions_fetched_updates_cache_on_success() {
    let mut app = test_app_with_two_environments();
    app.main_state_mut().available_versions.loading = true;
    app.main_state_mut().available_versions.remote.request_seq = 7;

    app.handle_remote_versions_fetched(
        7,
        Ok(vec![remote("v22.10.0", true), remote("v22.9.0", false)]),
    );

    let state = app.main_state();
    assert!(!state.available_versions.loading);
    assert!(state.available_versions.remote.error.is_none());
    assert_eq!(state.available_versions.versions.len(), 2);
    assert_eq!(
        state.available_versions.latest_by_major.get(&22),
        Some(&"v22.10.0".parse().expect("version parse"))
    );
    assert!(state.available_versions.fetched_at.is_some());
    assert!(!state.available_versions.loaded_from_disk);
}

#[test]
fn release_schedule_fetched_ignores_stale_request() {
    let mut app = test_app_with_two_environments();
    let baseline = sample_schedule();
    app.main_state_mut()
        .available_versions
        .schedule_fetch
        .request_seq = 3;
    app.main_state_mut().available_versions.schedule = Some(baseline.clone());

    app.handle_release_schedule_fetched(2, Ok(sample_schedule()));

    let state = app.main_state();
    assert_eq!(
        state
            .available_versions
            .schedule
            .as_ref()
            .expect("baseline schedule should remain")
            .versions
            .len(),
        baseline.versions.len()
    );
}

#[test]
fn release_schedule_fetched_sets_schedule_and_clears_error() {
    let mut app = test_app_with_two_environments();
    let state = app.main_state_mut();
    state.available_versions.schedule_fetch.request_seq = 5;
    state.available_versions.schedule_fetch.error = Some(AppError::version_fetch_failed(
        "Release schedule",
        "old error",
    ));

    app.handle_release_schedule_fetched(5, Ok(sample_schedule()));

    let state = app.main_state();
    assert!(state.available_versions.schedule.is_some());
    assert!(state.available_versions.schedule_fetch.error.is_none());
}

#[test]
fn version_metadata_fetched_ignores_stale_request() {
    let mut app = test_app_with_two_environments();
    let baseline = sample_metadata();
    app.main_state_mut()
        .available_versions
        .metadata_fetch
        .request_seq = 4;
    app.main_state_mut().available_versions.metadata = Some(baseline.clone());

    app.handle_version_metadata_fetched(3, Ok(sample_metadata()));

    let state = app.main_state();
    assert_eq!(
        state
            .available_versions
            .metadata
            .as_ref()
            .expect("baseline metadata should remain")
            .get("v22.10.0")
            .and_then(|meta| meta.npm.as_deref()),
        baseline
            .get("v22.10.0")
            .and_then(|meta| meta.npm.as_deref())
    );
}

#[test]
fn version_metadata_fetched_stores_metadata_on_success() {
    let mut app = test_app_with_two_environments();
    let state = app.main_state_mut();
    state.available_versions.metadata_fetch.request_seq = 8;
    state.available_versions.metadata = None;
    state.available_versions.metadata_fetch.error = Some(AppError::version_fetch_failed(
        "Version metadata",
        "old error",
    ));

    app.handle_version_metadata_fetched(8, Ok(sample_metadata()));

    let state = app.main_state();
    assert!(state.available_versions.metadata.is_some());
    assert!(state.available_versions.metadata_fetch.error.is_none());
}

#[test]
fn version_metadata_fetched_stores_error_on_failure() {
    let mut app = test_app_with_two_environments();
    app.main_state_mut()
        .available_versions
        .metadata_fetch
        .request_seq = 9;
    app.main_state_mut().available_versions.metadata = None;

    app.handle_version_metadata_fetched(
        9,
        Err(AppError::version_fetch_failed(
            "Version metadata",
            "metadata failed",
        )),
    );

    let state = app.main_state();
    assert!(matches!(
        state.available_versions.metadata_fetch.error,
        Some(AppError::VersionFetchFailed {
            resource: "Version metadata",
            ref details
        }) if details == &crate::error::AppErrorDetail::from("metadata failed")
    ));
}

#[test]
fn security_advisories_fetched_ignores_stale_request() {
    let mut app = test_app_with_two_environments();
    let baseline = sample_security_advisories();
    app.main_state_mut()
        .available_versions
        .security_fetch
        .request_seq = 6;
    app.main_state_mut().available_versions.security_advisories = Some(baseline.clone());

    app.handle_security_advisories_fetched(5, Ok(sample_security_advisories()));

    let state = app.main_state();
    assert_eq!(
        state
            .available_versions
            .security_advisories
            .as_ref()
            .expect("baseline advisories should remain")
            .get("163")
            .and_then(|advisory| advisory.cve.first())
            .map(String::as_str),
        baseline
            .get("163")
            .and_then(|advisory| advisory.cve.first())
            .map(String::as_str)
    );
}

#[test]
fn security_advisories_fetched_stores_data_on_success() {
    let mut app = test_app_with_two_environments();
    let state = app.main_state_mut();
    state.available_versions.security_fetch.request_seq = 10;
    state.available_versions.security_advisories = None;
    state.available_versions.security_fetch.error = Some(AppError::version_fetch_failed(
        "Security advisories",
        "old error",
    ));

    app.handle_security_advisories_fetched(10, Ok(sample_security_advisories()));

    let state = app.main_state();
    assert!(state.available_versions.security_advisories.is_some());
    assert!(state.available_versions.security_fetch.error.is_none());
    assert!(state.available_versions.security_last_checked_at.is_some());
}

#[test]
fn security_advisories_fetched_stores_error_on_failure() {
    let mut app = test_app_with_two_environments();
    app.main_state_mut()
        .available_versions
        .security_fetch
        .request_seq = 11;
    app.main_state_mut().available_versions.security_advisories = None;

    app.handle_security_advisories_fetched(
        11,
        Err(AppError::version_fetch_failed(
            "Security advisories",
            "security advisories failed",
        )),
    );

    let state = app.main_state();
    assert!(matches!(
        state.available_versions.security_fetch.error,
        Some(AppError::VersionFetchFailed {
            resource: "Security advisories",
            ref details
        }) if details == &crate::error::AppErrorDetail::from("security advisories failed")
    ));
    assert!(state.available_versions.security_last_checked_at.is_some());
}

#[test]
fn app_update_checked_sets_update_on_success() {
    let mut app = test_app_with_two_environments();
    let update = versi_core::AppUpdate {
        current_version: "0.9.0".to_string(),
        latest_version: "0.9.1".to_string(),
        release_url: "https://example.com/release".to_string(),
        release_notes: Some("notes".to_string()),
        download_url: Some("https://example.com/download".to_string()),
        download_size: Some(1234),
        download_sha256: Some(
            "50639d63848d275a7efcd04478de62ca0df8f35dfd75be490e4fcae667ecd436".to_string(),
        ),
    };

    let _ = app.handle_app_update_checked(Ok(Some(update.clone())));

    let state = app.main_state();
    assert_eq!(
        state
            .app_update
            .as_ref()
            .map(|value| value.latest_version.as_str()),
        Some("0.9.1")
    );
}

#[test]
fn app_update_checked_ignores_result_when_update_checks_are_disabled() {
    let mut app = test_app_with_two_environments();
    app.settings.app_update_behavior = AppUpdateBehavior::DoNotCheck;
    let state = app.main_state_mut();
    state.app_update_check_in_flight = true;
    state.app_update = Some(versi_core::AppUpdate {
        current_version: "0.9.0".to_string(),
        latest_version: "0.9.1".to_string(),
        release_url: "https://example.com/release".to_string(),
        release_notes: None,
        download_url: Some("https://example.com/download".to_string()),
        download_size: Some(42),
        download_sha256: Some(
            "50639d63848d275a7efcd04478de62ca0df8f35dfd75be490e4fcae667ecd436".to_string(),
        ),
    });

    let _ = app.handle_app_update_checked(Ok(Some(versi_core::AppUpdate {
        current_version: "0.9.0".to_string(),
        latest_version: "0.9.2".to_string(),
        release_url: "https://example.com/release".to_string(),
        release_notes: None,
        download_url: Some("https://example.com/download".to_string()),
        download_size: Some(84),
        download_sha256: Some(
            "50639d63848d275a7efcd04478de62ca0df8f35dfd75be490e4fcae667ecd436".to_string(),
        ),
    })));

    let state = app.main_state();
    assert!(state.app_update.is_none());
    assert!(!state.app_update_check_in_flight);
}

#[test]
fn check_for_app_update_marks_check_in_flight_when_enabled() {
    let mut app = test_app_with_two_environments();
    app.settings.app_update_behavior = AppUpdateBehavior::CheckPeriodically;

    let _ = app.handle_check_for_app_update();

    let state = app.main_state();
    assert!(state.app_update_check_in_flight);
}

#[test]
fn backend_update_checked_sets_update_on_success() {
    let mut app = test_app_with_two_environments();
    let update = versi_backend::BackendUpdate {
        current_version: "1.0.0".to_string(),
        latest_version: "1.1.0".to_string(),
        release_url: "https://example.com/backend".to_string(),
    };

    app.handle_backend_update_checked(Ok(Some(update.clone())));

    let state = app.main_state();
    assert_eq!(
        state
            .backend_update
            .as_ref()
            .map(|value| value.latest_version.as_str()),
        Some("1.1.0")
    );
}

#[test]
fn fetch_release_schedule_cancels_previous_token() {
    let mut app = test_app_with_two_environments();
    let old_token = CancellationToken::new();
    app.main_state_mut()
        .available_versions
        .schedule_fetch
        .cancel_token = Some(old_token.clone());

    let _ = app.handle_fetch_release_schedule();

    assert!(old_token.is_cancelled());
    let state = app.main_state();
    assert!(
        state
            .available_versions
            .schedule_fetch
            .cancel_token
            .is_some()
    );
}

#[test]
fn fetch_version_metadata_cancels_previous_token() {
    let mut app = test_app_with_two_environments();
    let old_token = CancellationToken::new();
    app.main_state_mut()
        .available_versions
        .metadata_fetch
        .cancel_token = Some(old_token.clone());

    let _ = app.handle_fetch_version_metadata();

    assert!(old_token.is_cancelled());
    let state = app.main_state();
    assert!(
        state
            .available_versions
            .metadata_fetch
            .cancel_token
            .is_some()
    );
}

#[test]
fn fetch_security_advisories_cancels_previous_token() {
    let mut app = test_app_with_two_environments();
    let old_token = CancellationToken::new();
    app.main_state_mut()
        .available_versions
        .security_fetch
        .cancel_token = Some(old_token.clone());

    let _ = app.handle_fetch_security_advisories();

    assert!(old_token.is_cancelled());
    let state = app.main_state();
    assert!(
        state
            .available_versions
            .security_fetch
            .cancel_token
            .is_some()
    );
}

#[test]
fn fetch_remote_versions_cancels_previous_token_when_loading() {
    let mut app = test_app_with_two_environments();
    let old_token = CancellationToken::new();
    app.main_state_mut().available_versions.loading = true;
    app.main_state_mut().available_versions.remote.cancel_token = Some(old_token.clone());

    let _ = app.handle_fetch_remote_versions();

    assert!(old_token.is_cancelled());
    let state = app.main_state();
    assert!(state.available_versions.loading);
    assert!(state.available_versions.remote.cancel_token.is_some());
}
