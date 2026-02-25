//! Remote version fetching, release schedule, and update checks.
//!
//! Handles messages: `RemoteVersionsFetched`, `ReleaseScheduleFetched`,
//! `AppUpdateChecked`, `BackendUpdateChecked`

use iced::Task;

use crate::error::AppError;
use crate::message::Message;

use super::Versi;

mod cache_save;
mod fetch_handlers;
mod update_checks;

impl Versi {
    pub(super) fn handle_fetch_remote_versions(&mut self) -> Task<Message> {
        fetch_handlers::handle_fetch_remote_versions(self)
    }

    pub(super) fn handle_remote_versions_fetched(
        &mut self,
        request_seq: u64,
        result: Result<Vec<versi_backend::RemoteVersion>, AppError>,
    ) {
        fetch_handlers::handle_remote_versions_fetched(self, request_seq, result);
    }

    pub(super) fn handle_fetch_release_schedule(&mut self) -> Task<Message> {
        fetch_handlers::handle_fetch_release_schedule(self)
    }

    pub(super) fn handle_release_schedule_fetched(
        &mut self,
        request_seq: u64,
        result: Result<versi_core::ReleaseSchedule, AppError>,
    ) {
        fetch_handlers::handle_release_schedule_fetched(self, request_seq, result);
    }

    pub(super) fn handle_fetch_version_metadata(&mut self) -> Task<Message> {
        fetch_handlers::handle_fetch_version_metadata(self)
    }

    pub(super) fn handle_version_metadata_fetched(
        &mut self,
        request_seq: u64,
        result: Result<std::collections::HashMap<String, versi_core::VersionMeta>, AppError>,
    ) {
        fetch_handlers::handle_version_metadata_fetched(self, request_seq, result);
    }

    pub(super) fn handle_fetch_security_advisories(&mut self) -> Task<Message> {
        fetch_handlers::handle_fetch_security_advisories(self)
    }

    pub(super) fn handle_security_advisories_fetched(
        &mut self,
        request_seq: u64,
        result: Result<std::collections::HashMap<String, versi_core::SecurityAdvisory>, AppError>,
    ) {
        fetch_handlers::handle_security_advisories_fetched(self, request_seq, result);
    }

    pub(super) fn handle_check_for_app_update(&mut self) -> Task<Message> {
        update_checks::handle_check_for_app_update(self)
    }

    pub(super) fn handle_app_update_checked(
        &mut self,
        result: Result<Option<versi_core::AppUpdate>, AppError>,
    ) -> Task<Message> {
        update_checks::handle_app_update_checked(self, result)
    }

    pub(super) fn handle_check_for_backend_update(&mut self) -> Task<Message> {
        update_checks::handle_check_for_backend_update(self)
    }

    pub(super) fn handle_backend_update_checked(
        &mut self,
        result: Result<Option<versi_backend::BackendUpdate>, AppError>,
    ) {
        update_checks::handle_backend_update_checked(self, result);
    }
}

#[cfg(test)]
mod tests;
