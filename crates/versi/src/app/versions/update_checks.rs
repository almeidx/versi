use std::time::Duration;

use iced::Task;
use log::debug;
use versi_backend::BackendDetection;
use versi_core::check_for_update;

use crate::error::AppError;
use crate::message::Message;
use crate::settings::AppUpdateBehavior;
use crate::state::{AppState, AppUpdateState};

use super::super::Versi;
use super::super::async_helpers::run_with_timeout;

pub(super) fn handle_check_for_app_update(app: &mut Versi) -> Task<Message> {
    if app.settings.app_update_behavior == AppUpdateBehavior::DoNotCheck {
        return Task::none();
    }

    if let AppState::Main(state) = &mut app.state {
        if state.app_update_check_in_flight {
            return Task::none();
        }
        state.app_update_check_in_flight = true;
    }

    let current_version = env!("CARGO_PKG_VERSION");
    let client = app.http_client.clone();
    let fetch_timeout = Duration::from_secs(app.settings.fetch_timeout_secs);
    Task::perform(
        async move {
            run_with_timeout(
                fetch_timeout,
                "Check for app update",
                check_for_update(&client, current_version),
                |error| AppError::update_check_failed("App", error),
            )
            .await
        },
        |result| Message::AppUpdateChecked(Box::new(result)),
    )
}

pub(super) fn handle_app_update_checked(
    app: &mut Versi,
    result: Result<Option<versi_core::AppUpdate>, AppError>,
) -> Task<Message> {
    if let AppState::Main(state) = &mut app.state {
        state.app_update_check_in_flight = false;
        state.app_update_last_checked_at = Some(std::time::Instant::now());

        if app.settings.app_update_behavior == AppUpdateBehavior::DoNotCheck {
            state.app_update = None;
            return Task::none();
        }

        match result {
            Ok(update) => {
                let should_auto_start = app.settings.app_update_behavior
                    == AppUpdateBehavior::AutomaticallyUpdate
                    && update.as_ref().is_some_and(|u| u.download_url.is_some())
                    && matches!(
                        state.app_update_state,
                        AppUpdateState::Idle | AppUpdateState::Failed(_)
                    );
                state.app_update = update;
                if should_auto_start {
                    return Task::done(Message::StartAppUpdate);
                }
            }
            Err(e) => debug!("App update check failed: {e}"),
        }
    }
    Task::none()
}

pub(super) fn handle_check_for_backend_update(app: &mut Versi) -> Task<Message> {
    if let AppState::Main(state) = &app.state
        && let Some(version) = &state.active_environment().backend_version
    {
        let version = version.clone();
        let client = app.http_client.clone();
        let provider = app.provider_for_kind(state.backend_name);
        let detection = BackendDetection {
            found: true,
            path: Some(app.backend_path.clone()),
            version: Some(version.clone()),
            in_path: app.backend_in_path,
            data_dir: app.backend_dir.clone(),
        };
        return Task::perform(
            async move {
                provider
                    .check_for_update(&client, &version, &detection)
                    .await
                    .map_err(|error| AppError::update_check_failed("Backend", error))
            },
            |result| Message::BackendUpdateChecked(Box::new(result)),
        );
    }
    Task::none()
}

pub(super) fn handle_backend_update_checked(
    app: &mut Versi,
    result: Result<Option<versi_backend::BackendUpdate>, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        match result {
            Ok(update) => state.backend_update = update,
            Err(e) => debug!("Backend update check failed: {e}"),
        }
    }
}
