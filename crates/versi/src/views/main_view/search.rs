use iced::widget::{Space, button, container, text_input, tooltip};
use iced::{Element, Length};

use crate::icon;
use crate::message::Message;
use crate::state::MainState;
use crate::theme::styles;
use crate::widgets::helpers::styled_tooltip;

pub const SEARCH_INPUT_ID: &str = "search-input";

pub(super) fn search_bar_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    let input = text_input(
        "Search versions (e.g., '22', 'lts', 'lts/iron', 'latest')...",
        &state.search_query,
    )
    .id(SEARCH_INPUT_ID)
    .on_input(Message::SearchChanged)
    .padding(14)
    .size(14)
    .style(styles::search_input);

    let clear_btn: Element<Message> = if state.search_query.is_empty() {
        Space::new().into()
    } else {
        styled_tooltip(
            button(icon::close(14.0))
                .on_press(Message::SearchChanged(String::new()))
                .style(styles::ghost_button)
                .padding([6, 10]),
            "Clear search",
            tooltip::Position::Left,
        )
    };

    iced::widget::stack![
        input,
        container(clear_btn)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding::new(0.0).right(4.0)),
    ]
    .into()
}
