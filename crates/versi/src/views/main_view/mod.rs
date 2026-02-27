mod banners;
mod context_menu;
mod header;
mod modals;
pub mod search;
pub mod tabs;

use iced::Element;
use iced::Length;
use iced::widget::{Space, column, container, mouse_area, row, scrollable};

use crate::message::Message;
use crate::settings::AppSettings;
use crate::state::MainState;
use crate::theme::styles;
use crate::widgets::{toast_container, version_list};

fn top_section(state: &MainState) -> iced::widget::Column<'_, Message> {
    let header = header::header_view(state);
    let search_bar = search::search_bar_view(state);

    let mut section = column![header, search_bar].spacing(12);

    if !state.search_query.is_empty() {
        section = section.push(search::filter_chips_view(&state.active_filters));
    }

    if state.search_query.is_empty()
        && let Some(banner_content) = banners::contextual_banners(state)
    {
        section = section.push(banner_content);
    }

    section
}

fn with_bulk_progress_overlay<'a>(
    state: &'a MainState,
    main_content: Element<'a, Message>,
    content_padding: iced::Padding,
    overlay_right_inset: f32,
) -> Element<'a, Message> {
    if let Some(progress_banner) = banners::bulk_operation_progress_banner(state) {
        let bottom_overlay = container(row![
            container(progress_banner)
                .padding(iced::Padding {
                    top: 0.0,
                    right: overlay_right_inset,
                    bottom: content_padding.bottom,
                    left: content_padding.left,
                })
                .width(Length::Fill),
            Space::new().width(Length::Fixed(styles::OVERLAY_SCROLLBAR_LANE_WIDTH)),
        ])
        .align_y(iced::alignment::Vertical::Bottom)
        .width(Length::Fill)
        .height(Length::Fill);

        iced::widget::stack![main_content, bottom_overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        main_content
    }
}

pub fn view<'a>(
    state: &'a MainState,
    settings: &'a AppSettings,
    has_tabs: bool,
) -> Element<'a, Message> {
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
        security_findings: &state.security_findings_by_version,
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

    let content_padding =
        crate::views::content_padding(has_tabs).right(crate::theme::tokens::INSET_RIGHT);
    let content_right_inset =
        content_padding.right + crate::theme::tokens::SCROLL_CONTENT_RIGHT_INSET;
    let overlay_right_inset = (content_right_inset - styles::OVERLAY_SCROLLBAR_LANE_WIDTH).max(0.0);

    let scroll_top_section = container(top_section(state)).padding(iced::Padding {
        top: 0.0,
        right: crate::theme::tokens::SCROLL_CONTENT_RIGHT_INSET,
        bottom: 12.0,
        left: 0.0,
    });

    let scroll_content = column![
        scroll_top_section,
        container(version_list).padding(
            iced::Padding::new(0.0).right(crate::theme::tokens::SCROLL_CONTENT_RIGHT_INSET)
        ),
    ]
    .spacing(0)
    .padding(content_padding)
    .width(Length::Fill);

    let main_scrollable = scrollable(scroll_content)
        .direction(iced::widget::scrollable::Direction::Vertical(
            styles::overlay_scrollbar(),
        ))
        .style(styles::overlay_scrollable)
        .width(Length::Fill)
        .height(Length::Fill);

    let fixed_top_overlay = iced::widget::opaque(row![
        container(top_section(state))
            .padding(iced::Padding {
                top: content_padding.top,
                right: overlay_right_inset,
                bottom: 12.0,
                left: content_padding.left,
            })
            .style(styles::page_background_overlay)
            .width(Length::Fill),
        Space::new().width(Length::Fixed(styles::OVERLAY_SCROLLBAR_LANE_WIDTH)),
    ]);

    let main_content = iced::widget::stack![main_scrollable, fixed_top_overlay]
        .width(Length::Fill)
        .height(Length::Fill);

    let with_bulk_progress_overlay = with_bulk_progress_overlay(
        state,
        main_content.into(),
        content_padding,
        overlay_right_inset,
    );

    let with_cursor_tracking: Element<Message> = mouse_area(with_bulk_progress_overlay)
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
