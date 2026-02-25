use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::icon;
use crate::message::Message;
use crate::state::MainState;
use crate::theme::styles;
use crate::widgets::helpers::nav_icons;

pub fn view(state: &MainState, has_tabs: bool) -> Element<'_, Message> {
    let content_padding = super::content_padding(has_tabs).right(crate::theme::tokens::INSET_RIGHT);
    let header = row![
        text("About").size(14),
        Space::new().width(Length::Fill),
        nav_icons(&state.view, state.refresh_rotation),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let content = column![
        text(format!("Versi v{}", env!("CARGO_PKG_VERSION"))).size(14),
        Space::new().height(4),
        text("A native GUI for managing Node.js versions")
            .size(12)
            .color(crate::theme::tokens::TEXT_MUTED),
        Space::new().height(12),
        row![
            button(
                row![text("GitHub").size(12), icon::arrow_up_right(12.0),]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
            .on_press(Message::OpenLink(
                "https://github.com/almeidx/versi".to_string()
            ))
            .style(styles::secondary_button)
            .padding([6, 12]),
            button(
                row![text("fnm").size(12), icon::arrow_up_right(12.0),]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
            .on_press(Message::OpenLink(
                "https://github.com/Schniz/fnm".to_string()
            ))
            .style(styles::secondary_button)
            .padding([6, 12]),
        ]
        .spacing(8),
    ]
    .spacing(4)
    .width(Length::Fill);

    let scroll_content = column![header, Space::new().height(12), content]
        .spacing(0)
        .padding(content_padding)
        .width(Length::Fill);

    scrollable(
        container(scroll_content).padding(
            iced::Padding::new(0.0).right(crate::theme::tokens::SCROLL_CONTENT_RIGHT_INSET),
        ),
    )
    .direction(iced::widget::scrollable::Direction::Vertical(
        styles::overlay_scrollbar(),
    ))
    .style(styles::overlay_scrollable)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
