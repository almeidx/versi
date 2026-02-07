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
            self.update_tray_menu();
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
            self.update_tray_menu();
            platform::set_dock_visible(true);
            Task::batch([
                iced::window::set_mode(id, iced::window::Mode::Windowed),
                iced::window::minimize(id, false),
                iced::window::gain_focus(id),
            ])
        } else if self.pending_minimize {
            self.pending_minimize = false;
            self.window_visible = false;
            self.update_tray_menu();
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
                x: pos.x as i32,
                y: pos.y as i32,
            });
            if let Err(e) = self.settings.save() {
                log::error!("Failed to save settings: {e}");
            }
        }
    }
}
