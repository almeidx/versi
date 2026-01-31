use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::icon;
use crate::message::Message;
use crate::state::MainState;
use crate::theme::styles;
use crate::widgets::helpers::nav_icons;

pub fn view<'a>(state: &'a MainState) -> Element<'a, Message> {
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
            .color(iced::Color::from_rgb8(142, 142, 147)),
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

    column![
        container(header).padding(iced::Padding::new(0.0).right(24.0)),
        Space::new().height(12),
        scrollable(content.padding(iced::Padding::default().right(24.0))).height(Length::Fill),
    ]
    .spacing(0)
    .padding(iced::Padding::new(24.0).top(12.0).right(0.0))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
