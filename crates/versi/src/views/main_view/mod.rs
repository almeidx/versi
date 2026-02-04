mod banners;
mod header;
mod modals;
pub mod search;
pub mod tabs;

use iced::Element;
use iced::widget::{column, container};

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
    let version_list = version_list::view(
        state.active_environment(),
        &state.search_query,
        &state.available_versions.versions,
        &state.available_versions.latest_by_major,
        state.available_versions.schedule.as_ref(),
        &state.operation_queue,
        hovered,
        settings.search_results_limit,
    );

    let right_inset = iced::Padding::new(0.0).right(24.0);
    let mut content_column = column![
        container(header).padding(right_inset),
        container(search_bar).padding(right_inset),
    ]
    .spacing(12);

    if state.search_query.is_empty()
        && let Some(banner_content) = banners::contextual_banners(state)
    {
        content_column = content_column.push(container(banner_content).padding(right_inset));
    }

    content_column = content_column.push(version_list);

    let content_padding = if has_tabs {
        iced::Padding::new(24.0).right(0.0)
    } else {
        iced::Padding::new(24.0).top(12.0).right(0.0)
    };
    let main_content = content_column.padding(content_padding);

    let main_column = column![main_content].spacing(0);

    let with_modal: Element<Message> = if let Some(modal) = &state.modal {
        modals::modal_overlay(main_column.into(), modal, state, settings)
    } else {
        main_column.into()
    };

    toast_container::view(with_modal, &state.toasts, settings.max_visible_toasts)
}
