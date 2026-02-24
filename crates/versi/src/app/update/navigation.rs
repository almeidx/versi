use iced::Task;

use crate::message::Message;
use crate::state::{AppState, MainViewKind};

use super::super::Versi;

impl Versi {
    pub(super) fn dispatch_navigation(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::Initialized(result) => Ok(self.handle_initialized(*result)),
            Message::EnvironmentLoaded {
                env_id,
                request_seq,
                result,
            } => Ok(self.handle_environment_loaded(&env_id, request_seq, result)),
            Message::StartBackgroundEnvironmentPreload { env_id } => {
                Ok(self.handle_start_background_environment_preload(&env_id))
            }
            Message::RefreshEnvironment => Ok(self.handle_refresh_environment()),
            Message::FocusSearch => Ok(self.focus_search()),
            other => self.dispatch_navigation_selection(other),
        }
    }

    fn dispatch_navigation_selection(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::SelectPreviousVersion => {
                self.move_version_selection(false);
                Ok(Task::none())
            }
            Message::SelectNextVersion => {
                self.move_version_selection(true);
                Ok(Task::none())
            }
            Message::ActivateSelectedVersion => Ok(self.activate_hovered_version()),
            Message::VersionGroupToggled { major } => {
                self.handle_version_group_toggled(major);
                Ok(Task::none())
            }
            Message::SearchChanged(query) => {
                self.handle_search_changed(query);
                Ok(Task::none())
            }
            Message::SearchFilterToggled(filter) => {
                self.handle_search_filter_toggled(filter);
                Ok(Task::none())
            }
            other => self.dispatch_navigation_data(other),
        }
    }

    fn dispatch_navigation_data(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::FetchRemoteVersions => Ok(self.handle_fetch_remote_versions()),
            Message::RemoteVersionsFetched {
                request_seq,
                result,
            } => {
                self.handle_remote_versions_fetched(request_seq, result);
                Ok(Task::none())
            }
            Message::ReleaseScheduleFetched {
                request_seq,
                result,
            } => {
                self.handle_release_schedule_fetched(request_seq, *result);
                Ok(Task::none())
            }
            Message::VersionMetadataFetched {
                request_seq,
                result,
            } => {
                self.handle_version_metadata_fetched(request_seq, *result);
                Ok(Task::none())
            }
            Message::ShowVersionDetail(version) => {
                if let AppState::Main(state) = &mut self.state {
                    state.modal = Some(crate::state::Modal::VersionDetail { version });
                }
                Ok(Task::none())
            }
            Message::CloseModal => {
                self.close_modal_or_return_to_versions();
                Ok(Task::none())
            }
            Message::OpenChangelog(version) => Ok(super::open_url_task(format!(
                "https://nodejs.org/en/blog/release/{version}"
            ))),
            Message::EnvironmentSelected(idx) => Ok(self.handle_environment_selected(idx)),
            Message::SelectNextEnvironment => Ok(self.select_environment_by_step(true)),
            Message::SelectPreviousEnvironment => Ok(self.select_environment_by_step(false)),
            Message::FetchReleaseSchedule => Ok(self.handle_fetch_release_schedule()),
            Message::FetchVersionMetadata => Ok(self.handle_fetch_version_metadata()),
            other => Err(Box::new(other)),
        }
    }

    fn focus_search(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.view = MainViewKind::Versions;
        }
        iced::widget::operation::focus(iced::widget::Id::new(
            crate::views::main_view::search::SEARCH_INPUT_ID,
        ))
    }

    fn move_version_selection(&mut self, next: bool) {
        if let AppState::Main(state) = &mut self.state
            && state.view == MainViewKind::Versions
            && state.modal.is_none()
        {
            let versions = state.navigable_versions(self.settings.search_results_limit);
            if versions.is_empty() {
                return;
            }

            let new_idx = match &state.hovered_version {
                Some(current) => versions.iter().position(|v| v == current).map_or(0, |i| {
                    if next {
                        (i + 1).min(versions.len() - 1)
                    } else {
                        i.saturating_sub(1)
                    }
                }),
                None => {
                    if next {
                        0
                    } else {
                        versions.len() - 1
                    }
                }
            };
            state.hovered_version = Some(versions[new_idx].clone());
        }
    }

    fn activate_hovered_version(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &self.state
            && state.view == MainViewKind::Versions
            && state.modal.is_none()
            && let Some(version) = state.hovered_version.clone()
        {
            if state.is_version_installed(&version) {
                return self.update(Message::SetDefault(version));
            }
            return self.update(Message::StartInstall(version));
        }
        Task::none()
    }

    fn select_environment_by_step(&mut self, next: bool) -> Task<Message> {
        if let AppState::Main(state) = &self.state
            && state.environments.len() > 1
        {
            let target = if next {
                (state.active_environment_idx + 1) % state.environments.len()
            } else if state.active_environment_idx == 0 {
                state.environments.len() - 1
            } else {
                state.active_environment_idx - 1
            };
            return self.handle_environment_selected(target);
        }
        Task::none()
    }

    fn close_modal_or_return_to_versions(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            if state.modal.is_some() {
                state.modal = None;
            } else if state.view == MainViewKind::About || state.view == MainViewKind::Settings {
                state.view = MainViewKind::Versions;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use versi_backend::{InstalledVersion, NodeVersion, RemoteVersion};
    use versi_platform::EnvironmentId;

    use super::super::super::test_app_with_two_environments;
    use super::*;
    use crate::state::{MainViewKind, Modal};

    fn installed(version: &str, is_default: bool) -> InstalledVersion {
        InstalledVersion {
            version: version.parse().expect("test version should parse"),
            is_default,
            lts_codename: None,
            install_date: None,
            disk_size: None,
        }
    }

    #[test]
    fn dispatch_navigation_returns_err_for_unhandled_message() {
        let mut app = test_app_with_two_environments();

        let result = app.dispatch_navigation(Message::NoOp);

        assert!(matches!(result, Err(other) if matches!(*other, Message::NoOp)));
    }

    #[test]
    fn dispatch_navigation_handles_background_preload_message() {
        let mut app = test_app_with_two_environments();
        let env_id = EnvironmentId::Wsl {
            distro: "Ubuntu".to_string(),
            backend_path: "/home/user/.nvm/nvm.sh".to_string(),
        };

        let result = app.dispatch_navigation(Message::StartBackgroundEnvironmentPreload { env_id });

        assert!(result.is_ok());
    }

    #[test]
    fn show_version_detail_sets_modal() {
        let mut app = test_app_with_two_environments();

        let _ = app.dispatch_navigation(Message::ShowVersionDetail("v20.11.0".to_string()));

        let state = app.main_state();
        assert!(matches!(
            state.modal,
            Some(Modal::VersionDetail { ref version }) if version == "v20.11.0"
        ));
    }

    #[test]
    fn close_modal_closes_open_modal_before_changing_view() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.modal = Some(Modal::KeyboardShortcuts);
        state.view = MainViewKind::About;

        app.close_modal_or_return_to_versions();

        let state = app.main_state();
        assert!(state.modal.is_none());
        assert_eq!(state.view, MainViewKind::About);
    }

    #[test]
    fn close_modal_returns_about_to_versions_when_no_modal() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.modal = None;
        state.view = MainViewKind::About;

        app.close_modal_or_return_to_versions();

        let state = app.main_state();
        assert_eq!(state.view, MainViewKind::Versions);
    }

    #[test]
    fn move_version_selection_advances_and_clamps_at_bounds() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.active_environment_mut().update_versions(vec![
            installed("v20.10.0", false),
            installed("v20.11.0", true),
        ]);
        state.view = MainViewKind::Versions;
        state.modal = None;

        app.move_version_selection(true);
        app.move_version_selection(true);
        app.move_version_selection(true);

        let state = app.main_state();
        assert_eq!(state.hovered_version.as_deref(), Some("v20.10.0"));
    }

    #[test]
    fn move_version_selection_navigates_search_results() {
        let mut app = test_app_with_two_environments();
        let state = app.main_state_mut();
        state.view = MainViewKind::Versions;
        state.modal = None;
        state.search_query = "24.1.0".to_string();
        state.available_versions.set_versions(vec![
            RemoteVersion {
                version: NodeVersion::new(24, 1, 0),
                lts_codename: None,
                is_latest: true,
            },
            RemoteVersion {
                version: NodeVersion::new(22, 11, 0),
                lts_codename: Some("Jod".to_string()),
                is_latest: false,
            },
        ]);

        app.move_version_selection(true);

        let state = app.main_state();
        assert_eq!(state.hovered_version.as_deref(), Some("v24.1.0"));
    }

    #[test]
    fn select_environment_by_step_wraps_backwards() {
        let mut app = test_app_with_two_environments();

        let _ = app.select_environment_by_step(false);

        let state = app.main_state();
        assert_eq!(state.active_environment_idx, 1);
    }

    #[test]
    fn dispatch_navigation_handles_fetch_version_metadata_message() {
        let mut app = test_app_with_two_environments();
        let before_seq = app
            .main_state()
            .available_versions
            .metadata_fetch
            .request_seq;

        let result = app.dispatch_navigation(Message::FetchVersionMetadata);

        assert!(result.is_ok());
        let state = app.main_state();
        assert_eq!(
            state.available_versions.metadata_fetch.request_seq,
            before_seq.wrapping_add(1)
        );
        assert!(
            state
                .available_versions
                .metadata_fetch
                .cancel_token
                .is_some()
        );
    }
}
