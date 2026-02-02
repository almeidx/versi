use std::time::{Duration, Instant};

use log::debug;

use iced::Task;

use versi_core::{check_for_update, fetch_release_schedule};

use crate::message::Message;
use crate::state::AppState;

const LIST_REMOTE_TIMEOUT: Duration = Duration::from_secs(30);

use super::Versi;

impl Versi {
    pub(super) fn handle_fetch_remote_versions(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.available_versions.loading {
                return Task::none();
            }
            state.available_versions.loading = true;

            let backend = state.backend.clone();

            return Task::perform(
                async move {
                    let delays = [0, 2, 5, 15];
                    let mut last_err = String::new();
                    for (attempt, &delay) in delays.iter().enumerate() {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        }
                        match tokio::time::timeout(LIST_REMOTE_TIMEOUT, backend.list_remote()).await
                        {
                            Err(_) => {
                                last_err = "Request timed out".to_string();
                                debug!("Remote versions fetch attempt {} timed out", attempt + 1,);
                            }
                            Ok(Ok(versions)) => return Ok(versions),
                            Ok(Err(e)) => {
                                last_err = e.to_string();
                                debug!(
                                    "Remote versions fetch attempt {} failed: {}",
                                    attempt + 1,
                                    last_err
                                );
                            }
                        }
                    }
                    Err(last_err)
                },
                Message::RemoteVersionsFetched,
            );
        }
        Task::none()
    }

    pub(super) fn handle_remote_versions_fetched(
        &mut self,
        result: Result<Vec<versi_backend::RemoteVersion>, String>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            state.available_versions.loading = false;
            match result {
                Ok(versions) => {
                    state.available_versions.versions = versions.clone();
                    state.available_versions.fetched_at = Some(Instant::now());
                    state.available_versions.error = None;
                    state.available_versions.loaded_from_disk = false;

                    let schedule = state.available_versions.schedule.clone();
                    tokio::task::spawn_blocking(move || {
                        let cache = crate::cache::DiskCache {
                            remote_versions: versions,
                            release_schedule: schedule,
                            cached_at: chrono::Utc::now(),
                        };
                        cache.save();
                    });
                }
                Err(error) => {
                    state.available_versions.error = Some(error);
                }
            }
        }
    }

    pub(super) fn handle_fetch_release_schedule(&mut self) -> Task<Message> {
        if let AppState::Main(_) = &self.state {
            let client = self.http_client.clone();

            return Task::perform(
                async move {
                    let delays = [0, 2, 5, 15];
                    let mut last_err = String::new();
                    for (attempt, &delay) in delays.iter().enumerate() {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        }
                        match fetch_release_schedule(&client).await {
                            Ok(schedule) => return Ok(schedule),
                            Err(e) => {
                                last_err = e;
                                debug!(
                                    "Release schedule fetch attempt {} failed: {}",
                                    attempt + 1,
                                    last_err
                                );
                            }
                        }
                    }
                    Err(last_err)
                },
                Message::ReleaseScheduleFetched,
            );
        }
        Task::none()
    }

    pub(super) fn handle_release_schedule_fetched(
        &mut self,
        result: Result<versi_core::ReleaseSchedule, String>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            match result {
                Ok(schedule) => {
                    state.available_versions.schedule = Some(schedule.clone());
                    state.available_versions.schedule_error = None;

                    let versions = state.available_versions.versions.clone();
                    tokio::task::spawn_blocking(move || {
                        let cache = crate::cache::DiskCache {
                            remote_versions: versions,
                            release_schedule: Some(schedule),
                            cached_at: chrono::Utc::now(),
                        };
                        cache.save();
                    });
                }
                Err(error) => {
                    debug!("Release schedule fetch failed: {}", error);
                    state.available_versions.schedule_error = Some(error);
                }
            }
        }
    }

    pub(super) fn handle_check_for_app_update(&mut self) -> Task<Message> {
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let client = self.http_client.clone();
        Task::perform(
            async move { check_for_update(&client, &current_version).await },
            Message::AppUpdateChecked,
        )
    }

    pub(super) fn handle_app_update_checked(
        &mut self,
        result: Result<Option<versi_core::AppUpdate>, String>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            match result {
                Ok(update) => state.app_update = update,
                Err(e) => debug!("App update check failed: {}", e),
            }
        }
    }

    pub(super) fn handle_check_for_backend_update(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &self.state
            && let Some(version) = &state.active_environment().backend_version
        {
            let version = version.clone();
            let client = self.http_client.clone();
            let provider = self.provider.clone();
            return Task::perform(
                async move { provider.check_for_update(&client, &version).await },
                Message::BackendUpdateChecked,
            );
        }
        Task::none()
    }

    pub(super) fn handle_backend_update_checked(
        &mut self,
        result: Result<Option<versi_backend::BackendUpdate>, String>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            match result {
                Ok(update) => state.backend_update = update,
                Err(e) => debug!("Backend update check failed: {}", e),
            }
        }
    }
}
