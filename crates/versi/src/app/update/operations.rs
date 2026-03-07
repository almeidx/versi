use iced::Task;

use crate::message::Message;

use super::super::Versi;

impl Versi {
    pub(super) fn dispatch_operations(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::StartInstall(version) => Ok(self.handle_start_install(version)),
            Message::InstallProgress { version, progress } => {
                Ok(self.handle_install_progress(version, progress))
            }
            Message::InstallComplete {
                version,
                success,
                error,
            } => Ok(self.handle_install_complete(version, success, error)),
            Message::RequestUninstall(version) => Ok(self.handle_uninstall(version)),
            Message::ConfirmUninstallDefault(version) => {
                Ok(self.handle_confirm_uninstall_default(version))
            }
            Message::UninstallComplete {
                version,
                success,
                error,
            } => Ok(self.handle_uninstall_complete(version, success, error)),
            Message::RequestBulkUpdateMajors => Ok(self.handle_request_bulk_update_majors()),
            Message::RequestBulkUninstallEOL => Ok(self.handle_request_bulk_uninstall_eol()),
            Message::RequestBulkUninstallMajor { major } => {
                Ok(self.handle_request_bulk_uninstall_major(major))
            }
            Message::ConfirmBulkUpdateMajors => Ok(self.handle_confirm_bulk_update_majors()),
            Message::ConfirmBulkUninstallEOL => Ok(self.handle_confirm_bulk_uninstall_eol()),
            Message::ConfirmBulkUninstallMajor { major } => {
                Ok(self.handle_confirm_bulk_uninstall_major(major))
            }
            Message::RequestBulkUninstallMajorExceptLatest { major } => {
                Ok(self.handle_request_bulk_uninstall_major_except_latest(major))
            }
            Message::ConfirmBulkUninstallMajorExceptLatest { major } => {
                Ok(self.handle_confirm_bulk_uninstall_major_except_latest(major))
            }
            Message::CancelBulkOperation => {
                self.handle_close_modal();
                Ok(Task::none())
            }
            Message::CancelBulkRun => Ok(self.handle_cancel_bulk_run()),
            Message::SetDefault(version) => Ok(self.handle_set_default(version)),
            Message::DefaultChanged { success, error } => {
                Ok(self.handle_default_changed(success, error))
            }
            other => Err(Box::new(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use versi_backend::InstalledVersion;

    use super::super::super::test_app_with_two_environments;
    use super::*;
    use crate::state::Modal;

    fn installed(version: &str) -> InstalledVersion {
        InstalledVersion {
            version: version.parse().expect("test version should parse"),
            is_default: false,
            lts_codename: None,
            disk_size: None,
        }
    }

    #[test]
    fn dispatch_operations_returns_err_for_unhandled_message() {
        let mut app = test_app_with_two_environments();

        let result = app.dispatch_operations(Message::NoOp);

        assert!(matches!(result, Err(other) if matches!(*other, Message::NoOp)));
    }

    #[test]
    fn cancel_bulk_operation_closes_modal() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut().modal = Some(Modal::KeyboardShortcuts);

        let _ = app.dispatch_operations(Message::CancelBulkOperation);

        let state = app.main_state();
        assert!(state.modal.is_none());
    }

    #[test]
    fn request_bulk_uninstall_major_opens_confirmation_modal() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut()
            .active_environment_mut()
            .update_versions(vec![
                installed("v20.11.0"),
                installed("v20.10.0"),
                installed("v18.19.0"),
            ]);

        let _ = app.dispatch_operations(Message::RequestBulkUninstallMajor { major: 20 });

        let state = app.main_state();
        assert!(matches!(
            state.modal,
            Some(Modal::ConfirmBulkUninstallMajor { major: 20, ref versions })
            if versions == &vec!["v20.11.0".to_string(), "v20.10.0".to_string()]
        ));
    }
}
