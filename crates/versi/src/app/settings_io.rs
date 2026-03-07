//! Settings import/export flows.
//!
//! Handles messages: `ExportSettings`, `SettingsExported`, `ImportSettings`, `SettingsImported`

use iced::Task;

use crate::error::AppError;
use crate::message::Message;
use crate::state::AppState;

use super::Versi;

impl Versi {
    pub(super) fn handle_export_settings(&self) -> Task<Message> {
        let settings = self.settings.clone();
        Task::perform(
            async move {
                let dialog = rfd::AsyncFileDialog::new()
                    .set_file_name("versi-settings.json")
                    .add_filter("JSON", &["json"])
                    .save_file()
                    .await;
                match dialog {
                    Some(handle) => export_settings_to_path(&settings, handle.path()).await,
                    None => Err(AppError::settings_dialog_cancelled()),
                }
            },
            Message::SettingsExported,
        )
    }

    pub(super) fn handle_settings_exported(
        &mut self,
        result: Result<std::path::PathBuf, AppError>,
    ) -> Task<Message> {
        if let Err(e) = result
            && !is_settings_dialog_cancelled(&e)
            && let AppState::Main(state) = &mut self.state
        {
            let id = state.next_toast_id();
            state.add_toast(crate::state::Toast::error(
                id,
                format!("Export failed: {e}"),
            ));
        }
        Task::none()
    }

    pub(super) fn handle_import_settings() -> Task<Message> {
        Task::perform(
            async {
                let dialog = rfd::AsyncFileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                    .await;
                match dialog {
                    Some(handle) => {
                        let imported = import_settings_from_path(handle.path()).await?;
                        imported
                            .save()
                            .map_err(|error| AppError::settings_import_failed("save", error))?;
                        Ok(())
                    }
                    None => Err(AppError::settings_dialog_cancelled()),
                }
            },
            Message::SettingsImported,
        )
    }

    pub(super) fn handle_settings_imported(
        &mut self,
        result: Result<(), AppError>,
    ) -> Task<Message> {
        match result {
            Ok(()) => {
                let old_settings = self.settings.clone();
                self.settings = crate::settings::AppSettings::load();
                return self.apply_imported_settings_side_effects(&old_settings);
            }
            Err(e) if !is_settings_dialog_cancelled(&e) => {
                if let AppState::Main(state) = &mut self.state {
                    let id = state.next_toast_id();
                    state.add_toast(crate::state::Toast::error(
                        id,
                        format!("Import failed: {e}"),
                    ));
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn apply_imported_settings_side_effects(
        &mut self,
        old: &crate::settings::AppSettings,
    ) -> Task<Message> {
        if old.debug_logging != self.settings.debug_logging {
            crate::logging::set_logging_enabled(self.settings.debug_logging);
        }

        if old.launch_at_login != self.settings.launch_at_login
            && let Err(e) = super::platform::set_launch_at_login(self.settings.launch_at_login)
        {
            log::error!("Failed to set launch at login: {e}");
        }

        if old.tray_behavior != self.settings.tray_behavior {
            use crate::settings::TrayBehavior;
            use crate::tray;

            if old.tray_behavior == TrayBehavior::Disabled
                && self.settings.tray_behavior != TrayBehavior::Disabled
            {
                if let Err(e) = tray::init_tray(self.settings.tray_behavior) {
                    log::error!("Failed to initialize tray: {e}");
                } else {
                    self.update_tray_menu();
                }
            } else if self.settings.tray_behavior == TrayBehavior::Disabled {
                tray::destroy_tray();
            }
        }

        Task::none()
    }
}

fn is_settings_dialog_cancelled(error: &AppError) -> bool {
    matches!(error, AppError::SettingsDialogCancelled)
}

async fn export_settings_to_path(
    settings: &crate::settings::AppSettings,
    path: &std::path::Path,
) -> Result<std::path::PathBuf, AppError> {
    let content = serde_json::to_string_pretty(settings)
        .map_err(|error| AppError::settings_export_failed("serialize", error))?;
    tokio::fs::write(path, content)
        .await
        .map_err(|error| AppError::settings_export_failed("write", error))?;
    Ok(path.to_path_buf())
}

async fn import_settings_from_path(
    path: &std::path::Path,
) -> Result<crate::settings::AppSettings, AppError> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|error| AppError::settings_import_failed("read", error))?;
    let mut settings: crate::settings::AppSettings = serde_json::from_str(&content)
        .map_err(|error| AppError::settings_import_failed("parse", error))?;
    settings.sanitize_in_place();
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{export_settings_to_path, import_settings_from_path};
    use crate::settings::{AppSettings, ThemeSetting, TrayBehavior};

    #[tokio::test]
    async fn export_then_import_roundtrips_settings_file() {
        let temp_dir = tempdir().expect("create temp dir");
        let export_path = temp_dir.path().join("settings.json");

        let mut settings = AppSettings::default();
        settings.theme = ThemeSetting::Dark;
        settings.tray_behavior = TrayBehavior::AlwaysRunning;
        settings.start_minimized = true;
        settings.fetch_timeout_secs = 42;
        settings.retry_delays_secs = vec![0, 1, 2];

        export_settings_to_path(&settings, &export_path)
            .await
            .expect("export settings");
        let imported = import_settings_from_path(&export_path)
            .await
            .expect("import settings");

        assert!(matches!(imported.theme, ThemeSetting::Dark));
        assert!(matches!(
            imported.tray_behavior,
            TrayBehavior::AlwaysRunning
        ));
        assert!(imported.start_minimized);
        assert_eq!(imported.fetch_timeout_secs, 42);
        assert_eq!(imported.retry_delays_secs, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn import_settings_rejects_invalid_json() {
        let temp_dir = tempdir().expect("create temp dir");
        let import_path = temp_dir.path().join("broken-settings.json");
        tokio::fs::write(&import_path, "{not valid json")
            .await
            .expect("write invalid payload");

        let error = import_settings_from_path(&import_path)
            .await
            .expect_err("expected parse failure");
        assert!(matches!(
            error,
            crate::error::AppError::SettingsImportFailed {
                action: "parse",
                ..
            }
        ));
    }
}
