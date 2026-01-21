use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::{Toast, ToastStatus};

pub fn view<'a>(content: Element<'a, Message>, toasts: &'a [Toast]) -> Element<'a, Message> {
    if toasts.is_empty() {
        return content;
    }

    let toast_elements: Vec<Element<Message>> =
        toasts.iter().map(|toast| toast_view(toast)).collect();

    let toast_column = container(column(toast_elements).spacing(8).align_x(Alignment::End))
        .padding(16)
        .width(Length::Shrink);

    let toast_layer = container(toast_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Bottom);

    iced::widget::stack![content, toast_layer].into()
}

fn toast_view<'a>(toast: &'a Toast) -> Element<'a, Message> {
    let (bg_color, text_color) = match toast.status {
        ToastStatus::Success => (iced::Color::from_rgb8(52, 199, 89), iced::Color::WHITE),
        ToastStatus::Error => (iced::Color::from_rgb8(255, 59, 48), iced::Color::WHITE),
        ToastStatus::Warning => (iced::Color::from_rgb8(255, 149, 0), iced::Color::WHITE),
        ToastStatus::Info => (iced::Color::from_rgb8(0, 122, 255), iced::Color::WHITE),
    };

    let mut content = row![text(&toast.message).size(14),].spacing(8);

    if toast.undo_action.is_some() {
        content = content.push(
            button(text("Undo").size(12))
                .on_press(Message::ToastUndo(toast.id))
                .style(|_theme, status| {
                    let base = iced::widget::button::Style {
                        background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                        text_color: iced::Color::WHITE,
                        border: iced::Border {
                            radius: 4.0.into(),
                            width: 1.0,
                            color: iced::Color::WHITE,
                        },
                        shadow: iced::Shadow::default(),
                        snap: false,
                    };

                    match status {
                        iced::widget::button::Status::Hovered => iced::widget::button::Style {
                            background: Some(iced::Background::Color(iced::Color {
                                a: 0.2,
                                ..iced::Color::WHITE
                            })),
                            ..base
                        },
                        _ => base,
                    }
                })
                .padding([4, 8]),
        );
    }

    content = content.push(
        button(text("×").size(14))
            .on_press(Message::ToastDismiss(toast.id))
            .style(|_theme, _status| iced::widget::button::Style {
                background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                text_color: iced::Color::WHITE,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: false,
            })
            .padding([0, 4]),
    );

    container(content.align_y(Alignment::Center))
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(bg_color)),
            text_color: Some(text_color),
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: iced::Shadow {
                color: iced::Color {
                    a: 0.2,
                    ..iced::Color::BLACK
                },
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            snap: false,
        })
        .padding([12, 16])
        .max_width(400)
        .into()
}
