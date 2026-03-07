use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::Task;
use log::debug;

use versi_core::{fetch_release_schedule, fetch_security_advisories, fetch_version_metadata};

use crate::error::AppError;
use crate::message::Message;
use crate::state::AppState;

use super::super::Versi;
use super::super::async_helpers::{retry_with_delays, run_with_timeout};
use super::cache_save::{
    enqueue_cache_save_release_schedule, enqueue_cache_save_remote_versions,
    enqueue_cache_save_security_advisories, enqueue_cache_save_version_metadata,
};

pub(super) fn handle_fetch_remote_versions(app: &mut Versi) -> Task<Message> {
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.loading = true;
        let (cancel_token, request_seq) = state.available_versions.remote.start();
        state.available_versions.remote.error = None;

        let backend = state.backend.clone();
        let fetch_timeout = Duration::from_secs(app.settings.fetch_timeout_secs);
        let retry_delays = app.settings.retry_delays_secs.clone();

        return Task::perform(
            async move {
                tokio::select! {
                    () = cancel_token.cancelled() => {
                        Err(AppError::operation_cancelled("Remote versions fetch"))
                    }
                    result = retry_with_delays("Remote versions fetch", &retry_delays, || {
                        let backend = backend.clone();
                        async move {
                            run_with_timeout(
                                fetch_timeout,
                                "Fetch remote versions",
                                backend.list_remote(),
                                |error| AppError::version_fetch_failed("Remote versions", error),
                            )
                            .await
                        }
                    }) => result
                }
            },
            move |result| Message::RemoteVersionsFetched {
                request_seq,
                result,
            },
        );
    }
    Task::none()
}

pub(super) fn handle_remote_versions_fetched(
    app: &mut Versi,
    request_seq: u64,
    result: Result<Vec<versi_backend::RemoteVersion>, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        if !state.available_versions.remote.accept(request_seq) {
            debug!(
                "Ignoring stale remote versions response: request_seq={} current_seq={}",
                request_seq, state.available_versions.remote.request_seq
            );
            return;
        }

        state.available_versions.loading = false;
        match result {
            Ok(versions) => {
                state.available_versions.set_versions(versions);
                state.available_versions.fetched_at = Some(Instant::now());
                state.available_versions.remote.error = None;
                state.available_versions.loaded_from_disk = false;

                let env = state.active_environment();
                let installed_majors: std::collections::HashSet<u32> = env
                    .installed_versions
                    .iter()
                    .map(|v| v.version.major)
                    .collect();
                let has_update = installed_majors.iter().any(|major| {
                    state
                        .available_versions
                        .latest_by_major
                        .get(major)
                        .is_some_and(|latest| !env.installed_set.contains(latest))
                });
                super::super::platform::set_update_badge(has_update);

                enqueue_cache_save_remote_versions(Arc::clone(&state.available_versions.versions));
            }
            Err(error) => {
                state.available_versions.remote.error = Some(error);
            }
        }

        state.recompute_banner_stats();
    }
}

pub(super) fn handle_fetch_release_schedule(app: &mut Versi) -> Task<Message> {
    if let AppState::Main(state) = &mut app.state {
        let (cancel_token, request_seq) = state.available_versions.schedule_fetch.start();
        state.available_versions.schedule_fetch.error = None;
        let client = app.http_client.clone();
        let fetch_timeout = Duration::from_secs(app.settings.fetch_timeout_secs);
        let retry_delays = app.settings.retry_delays_secs.clone();

        return Task::perform(
            async move {
                tokio::select! {
                    () = cancel_token.cancelled() => {
                        Err(AppError::operation_cancelled("Release schedule fetch"))
                    }
                    result = retry_with_delays("Release schedule fetch", &retry_delays, || {
                        let client = client.clone();
                        async move {
                            run_with_timeout(
                                fetch_timeout,
                                "Fetch release schedule",
                                fetch_release_schedule(&client),
                                |error| AppError::version_fetch_failed("Release schedule", error),
                            )
                            .await
                        }
                    }) => result
                }
            },
            move |result| Message::ReleaseScheduleFetched {
                request_seq,
                result: Box::new(result),
            },
        );
    }
    Task::none()
}

pub(super) fn handle_release_schedule_fetched(
    app: &mut Versi,
    request_seq: u64,
    result: Result<versi_core::ReleaseSchedule, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        if !state.available_versions.schedule_fetch.accept(request_seq) {
            debug!(
                "Ignoring stale release schedule response: request_seq={} current_seq={}",
                request_seq, state.available_versions.schedule_fetch.request_seq
            );
            return;
        }

        match result {
            Ok(schedule) => {
                enqueue_cache_save_release_schedule(schedule.clone());
                state.available_versions.schedule = Some(schedule);
                state.available_versions.schedule_fetch.error = None;
            }
            Err(error) => {
                debug!("Release schedule fetch failed: {error}");
                state.available_versions.schedule_fetch.error = Some(error);
            }
        }

        state.recompute_banner_stats();
    }
}

pub(super) fn handle_fetch_version_metadata(app: &mut Versi) -> Task<Message> {
    if let AppState::Main(state) = &mut app.state {
        let (cancel_token, request_seq) = state.available_versions.metadata_fetch.start();
        state.available_versions.metadata_fetch.error = None;
        let client = app.http_client.clone();
        let fetch_timeout = Duration::from_secs(app.settings.fetch_timeout_secs);
        let retry_delays = app.settings.retry_delays_secs.clone();

        return Task::perform(
            async move {
                tokio::select! {
                    () = cancel_token.cancelled() => {
                        Err(AppError::operation_cancelled("Version metadata fetch"))
                    }
                    result = retry_with_delays("Version metadata fetch", &retry_delays, || {
                        let client = client.clone();
                        async move {
                            run_with_timeout(
                                fetch_timeout,
                                "Fetch version metadata",
                                fetch_version_metadata(&client),
                                |error| AppError::version_fetch_failed("Version metadata", error),
                            )
                            .await
                        }
                    }) => result
                }
            },
            move |result| Message::VersionMetadataFetched {
                request_seq,
                result: Box::new(result),
            },
        );
    }
    Task::none()
}

pub(super) fn handle_version_metadata_fetched(
    app: &mut Versi,
    request_seq: u64,
    result: Result<std::collections::HashMap<String, versi_core::VersionMeta>, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        if !state.available_versions.metadata_fetch.accept(request_seq) {
            debug!(
                "Ignoring stale version metadata response: request_seq={} current_seq={}",
                request_seq, state.available_versions.metadata_fetch.request_seq
            );
            return;
        }

        match result {
            Ok(metadata) => {
                let metadata = Arc::new(metadata);
                enqueue_cache_save_version_metadata(Arc::clone(&metadata));
                state.available_versions.metadata = Some(metadata);
                state.available_versions.metadata_fetch.error = None;
            }
            Err(error) => {
                debug!("Version metadata fetch failed: {error}");
                state.available_versions.metadata_fetch.error = Some(error);
            }
        }
    }
}

pub(super) fn handle_fetch_security_advisories(app: &mut Versi) -> Task<Message> {
    if let AppState::Main(state) = &mut app.state {
        let (cancel_token, request_seq) = state.available_versions.security_fetch.start();
        state.available_versions.security_fetch.error = None;
        let client = app.http_client.clone();
        let fetch_timeout = Duration::from_secs(app.settings.fetch_timeout_secs);
        let retry_delays = app.settings.retry_delays_secs.clone();

        return Task::perform(
            async move {
                tokio::select! {
                    () = cancel_token.cancelled() => {
                        Err(AppError::operation_cancelled("Security advisories fetch"))
                    }
                    result = retry_with_delays("Security advisories fetch", &retry_delays, || {
                        let client = client.clone();
                        async move {
                            run_with_timeout(
                                fetch_timeout,
                                "Fetch security advisories",
                                fetch_security_advisories(&client),
                                |error| AppError::version_fetch_failed("Security advisories", error),
                            )
                            .await
                        }
                    }) => result
                }
            },
            move |result| Message::SecurityAdvisoriesFetched {
                request_seq,
                result: Box::new(result),
            },
        );
    }
    Task::none()
}

pub(super) fn handle_security_advisories_fetched(
    app: &mut Versi,
    request_seq: u64,
    result: Result<std::collections::HashMap<String, versi_core::SecurityAdvisory>, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        if !state.available_versions.security_fetch.accept(request_seq) {
            debug!(
                "Ignoring stale security advisories response: request_seq={} current_seq={}",
                request_seq, state.available_versions.security_fetch.request_seq
            );
            return;
        }

        state.available_versions.security_last_checked_at = Some(Instant::now());
        match result {
            Ok(advisories) => {
                let advisories = Arc::new(advisories);
                enqueue_cache_save_security_advisories(Arc::clone(&advisories));
                state.available_versions.security_advisories = Some(advisories);
                state.available_versions.security_fetch.error = None;
            }
            Err(error) => {
                debug!("Security advisories fetch failed: {error}");
                state.available_versions.security_fetch.error = Some(error);
            }
        }

        state.recompute_banner_stats();
    }
}
