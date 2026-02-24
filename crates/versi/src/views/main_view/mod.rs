mod banners;
mod context_menu;
mod header;
mod modals;
pub mod search;
pub mod tabs;

use iced::Element;
use iced::widget::{column, container, mouse_area};

use crate::message::Message;
use crate::settings::AppSettings;
use crate::state::MainState;
use crate::widgets::{toast_container, version_list};

pub fn view<'a>(
    state: &'a MainState,
    settings: &'a AppSettings,
    has_tabs: bool,
) -> Element<'a, Message> {
    let header = header::header_view(state);
    let search_bar = search::search_bar_view(state);
    let hovered = if state.modal.is_some() {
        &None
    } else {
        &state.hovered_version
    };
    let env = state.active_environment();
    let ctx = version_list::VersionListContext {
        schedule: state.available_versions.schedule.as_ref(),
        search_index: Some(&state.available_versions.search_index),
        operation_queue: &state.operation_queue,
        install_progress: &state.install_progress,
        hovered_version: hovered,
        metadata: state.available_versions.metadata.as_ref(),
        installed_set: &env.installed_set,
    };
    let version_list = version_list::view(
        env,
        &state.search_query,
        &state.available_versions.versions,
        &state.available_versions.latest_by_major,
        settings.search_results_limit,
        &state.active_filters,
        &ctx,
    );

    let right_inset = iced::Padding::new(0.0).right(crate::theme::tokens::INSET_RIGHT);
    let mut content_column = column![
        container(header).padding(right_inset),
        container(search_bar).padding(right_inset),
    ]
    .spacing(12);

    if !state.search_query.is_empty() {
        let chips = search::filter_chips_view(&state.active_filters);
        content_column = content_column.push(container(chips).padding(right_inset));
    }

    if state.search_query.is_empty()
        && let Some(banner_content) = banners::contextual_banners(state)
    {
        content_column = content_column.push(container(banner_content).padding(right_inset));
    }

    content_column = content_column.push(version_list);

    let main_content = content_column.padding(crate::views::content_padding(has_tabs));

    let main_column = column![main_content].spacing(0);

    let with_cursor_tracking: Element<Message> = mouse_area(main_column)
        .on_move(Message::VersionListCursorMoved)
        .into();

    let with_context_menu: Element<Message> = if let Some(menu) = &state.context_menu {
        context_menu::context_menu_overlay(with_cursor_tracking, menu)
    } else {
        with_cursor_tracking
    };

    let with_modal: Element<Message> = if let Some(modal) = &state.modal {
        modals::modal_overlay(with_context_menu, modal, state, settings)
    } else {
        with_context_menu
    };

    toast_container::view(with_modal, &state.toasts, settings.max_visible_toasts)
}
