use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Element, Length};

use crate::message::Message;
use crate::state::ContextMenu;
use crate::theme::styles;

pub(super) fn context_menu_overlay<'a>(
    content: Element<'a, Message>,
    menu: &ContextMenu,
    supports_uninstall: bool,
) -> Element<'a, Message> {
    let backdrop = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::CloseContextMenu)
    .on_right_press(Message::CloseContextMenu);

    let mut items: Vec<Element<Message>> = Vec::new();

    if menu.is_installed {
        if !menu.is_default {
            items.push(
                button(text("Set as Default").size(13))
                    .on_press(Message::SetDefault(menu.version.clone()))
                    .style(styles::context_menu_item)
                    .padding([6, 12])
                    .width(Length::Fill)
                    .into(),
            );
        }
        if supports_uninstall {
            items.push(
                button(text("Uninstall").size(13))
                    .on_press(Message::RequestUninstall(menu.version.clone()))
                    .style(styles::context_menu_item_danger)
                    .padding([6, 12])
                    .width(Length::Fill)
                    .into(),
            );
        }
    } else {
        items.push(
            button(text("Install").size(13))
                .on_press(Message::StartInstall(menu.version.clone()))
                .style(styles::context_menu_item)
                .padding([6, 12])
                .width(Length::Fill)
                .into(),
        );
    }

    if !items.is_empty() {
        items.push(
            container(Space::new().width(Length::Fill).height(1))
                .style(styles::context_menu_separator)
                .width(Length::Fill)
                .into(),
        );
    }

    items.push(
        button(text("Copy Version Number").size(13))
            .on_press(Message::CopyToClipboard(menu.version.clone()))
            .style(styles::context_menu_item)
            .padding([6, 12])
            .width(Length::Fill)
            .into(),
    );

    items.push(
        button(text("Open Changelog").size(13))
            .on_press(Message::OpenChangelog(menu.version.clone()))
            .style(styles::context_menu_item)
            .padding([6, 12])
            .width(Length::Fill)
            .into(),
    );

    let menu_column = column(items)
        .spacing(2)
        .width(crate::theme::tokens::CONTEXT_MENU_WIDTH);

    let menu_widget = mouse_area(
        container(menu_column)
            .style(styles::context_menu_container)
            .padding(4),
    )
    .on_press(Message::NoOp);

    let positioned = column![
        Space::new().height(menu.position.y),
        row![Space::new().width(menu.position.x), menu_widget,],
    ];

    iced::widget::stack![content, backdrop, positioned].into()
}
