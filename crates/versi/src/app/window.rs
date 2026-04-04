//! Window lifecycle: open, close, hide-to-tray, and geometry persistence.
//!
//! Handles messages: `WindowClose`, `WindowOpened`

use log::info;

use iced::Task;

use crate::message::Message;
use crate::settings::TrayBehavior;
use crate::tray;

use super::Versi;
use super::platform;

impl Versi {
    pub(super) fn handle_window_close(&mut self) -> Task<Message> {
        info!(
            "Window close: tray_behavior={:?}, tray_active={}",
            self.settings.tray_behavior,
            tray::is_tray_active()
        );
        self.save_window_geometry();
        if self.settings.tray_behavior == TrayBehavior::AlwaysRunning && tray::is_tray_active() {
            self.window_visible = false;
            self.update_tray_tooltip();
            if let Some(id) = self.window_id {
                platform::set_dock_visible(false);
                if platform::is_wayland() {
                    info!("Minimizing window (Wayland fallback)");
                    iced::window::minimize(id, true)
                } else {
                    info!("Hiding window to tray");
                    iced::window::set_mode(id, iced::window::Mode::Hidden)
                }
            } else {
                Task::none()
            }
        } else {
            info!("Exiting application");
            iced::exit()
        }
    }

    pub(super) fn handle_window_opened(&mut self, id: iced::window::Id) -> Task<Message> {
        self.window_id = Some(id);
        if self.pending_show {
            self.pending_show = false;
            self.pending_minimize = false;
            self.window_visible = true;
            self.update_tray_tooltip();
            platform::set_dock_visible(true);
            Task::batch([
                iced::window::set_mode(id, iced::window::Mode::Windowed),
                iced::window::minimize(id, false),
                iced::window::gain_focus(id),
            ])
        } else if self.pending_minimize {
            self.pending_minimize = false;
            self.window_visible = false;
            self.update_tray_tooltip();
            let hide_task = if platform::is_wayland() {
                iced::window::minimize(id, true)
            } else {
                iced::window::set_mode(id, iced::window::Mode::Hidden)
            };
            Task::batch([Task::done(Message::HideDockIcon), hide_task])
        } else {
            Task::none()
        }
    }

    pub(super) fn save_window_geometry(&mut self) {
        if let (Some(size), Some(pos)) = (self.window_size, self.window_position) {
            self.settings.window_geometry = Some(crate::settings::WindowGeometry {
                width: size.width,
                height: size.height,
                x: pos.x,
                y: pos.y,
            });
            self.save_settings_with_log_sync();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_app_with_two_environments;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn window_opened_with_pending_show_updates_visibility_flags() {
        let mut app = test_app_with_two_environments();
        app.pending_show = true;
        app.pending_minimize = true;
        app.window_visible = false;

        let id = iced::window::Id::unique();
        let _ = app.handle_window_opened(id);

        assert_eq!(app.window_id, Some(id));
        assert!(!app.pending_show);
        assert!(!app.pending_minimize);
        assert!(app.window_visible);
    }

    #[test]
    fn window_opened_with_pending_minimize_marks_window_hidden() {
        let mut app = test_app_with_two_environments();
        app.pending_show = false;
        app.pending_minimize = true;
        app.window_visible = true;

        let id = iced::window::Id::unique();
        let _ = app.handle_window_opened(id);

        assert_eq!(app.window_id, Some(id));
        assert!(!app.pending_minimize);
        assert!(!app.window_visible);
    }

    #[test]
    fn save_window_geometry_persists_in_memory_when_dimensions_present() {
        let mut app = test_app_with_two_environments();
        app.window_size = Some(iced::Size::new(1200.0, 800.0));
        app.window_position = Some(iced::Point::new(100.0, 150.0));

        app.save_window_geometry();

        let geometry = app
            .settings
            .window_geometry
            .as_ref()
            .expect("window geometry should be stored");
        assert_close(geometry.width, 1200.0);
        assert_close(geometry.height, 800.0);
        assert_close(geometry.x, 100.0);
        assert_close(geometry.y, 150.0);
    }

    #[test]
    fn save_window_geometry_noop_when_size_or_position_missing() {
        let mut app = test_app_with_two_environments();
        app.settings.window_geometry = None;
        app.window_size = Some(iced::Size::new(1200.0, 800.0));
        app.window_position = None;

        app.save_window_geometry();

        assert!(app.settings.window_geometry.is_none());
    }
}
