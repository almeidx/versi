mod navigation;
mod operations;
mod settings;
mod system;

use iced::Task;

use crate::message::Message;
use crate::state::AppState;

use super::{Versi, should_dismiss_context_menu};

type DispatchResult = Result<Task<Message>, Box<Message>>;

impl Versi {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        self.dismiss_context_menu_if_needed(&message);

        let message = match self.dispatch_navigation(message) {
            Ok(task) => return task,
            Err(message) => *message,
        };
        let message = match self.dispatch_operations(message) {
            Ok(task) => return task,
            Err(message) => *message,
        };
        let message = match self.dispatch_settings(message) {
            Ok(task) => return task,
            Err(message) => *message,
        };
        let message = match self.dispatch_system(message) {
            Ok(task) => return task,
            Err(message) => *message,
        };

        let _ = message;
        Task::none()
    }

    fn dismiss_context_menu_if_needed(&mut self, message: &Message) {
        if let AppState::Main(state) = &mut self.state
            && state.context_menu.is_some()
            && should_dismiss_context_menu(message)
        {
            state.context_menu = None;
        }
    }
}

pub(super) fn open_url_task(url: String) -> Task<Message> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        log::warn!("Blocked open request for non-HTTP URL: {url}");
        return Task::none();
    }
    Task::perform(
        async move {
            let _ = open::that(&url);
        },
        |()| Message::NoOp,
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_app_with_two_environments;
    use super::*;
    use crate::state::ContextMenu;

    fn context_menu() -> ContextMenu {
        ContextMenu {
            version: "v20.11.0".to_string(),
            is_installed: true,
            is_default: false,
            position: iced::Point::new(10.0, 20.0),
        }
    }

    #[test]
    fn dismiss_context_menu_clears_for_unrelated_messages() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut().context_menu = Some(context_menu());

        app.dismiss_context_menu_if_needed(&Message::RefreshEnvironment);

        let state = app.main_state();
        assert!(state.context_menu.is_none());
    }

    #[test]
    fn dismiss_context_menu_keeps_for_allowed_messages() {
        let mut app = test_app_with_two_environments();
        app.main_state_mut().context_menu = Some(context_menu());

        app.dismiss_context_menu_if_needed(&Message::Tick);
        app.dismiss_context_menu_if_needed(&Message::ShowContextMenu {
            version: "v20.11.0".to_string(),
            is_installed: true,
            is_default: false,
        });

        let state = app.main_state();
        assert!(state.context_menu.is_some());
    }

    #[test]
    fn dismiss_context_menu_ignores_non_main_state() {
        let mut app = test_app_with_two_environments();
        app.state = AppState::Loading;

        app.dismiss_context_menu_if_needed(&Message::RefreshEnvironment);

        assert!(matches!(app.state, AppState::Loading));
    }
}
