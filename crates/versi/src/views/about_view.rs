use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::backend_kind::BackendKind;
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

    let backend_summary = detected_backends_summary(&state.detected_backends);
    let content = column![
        text("Versi").size(24),
        text(format!("v{}", env!("CARGO_PKG_VERSION")))
            .size(13)
            .color(crate::theme::tokens::TEXT_MUTED),
        text("A native GUI for managing Node.js versions")
            .size(13)
            .color(crate::theme::tokens::TEXT_MUTED),
        Space::new().height(12),
        text("Project").size(13),
        row![
            link_button("GitHub", "https://github.com/almeidx/versi"),
            link_button("Releases", "https://github.com/almeidx/versi/releases"),
            link_button(
                "License",
                "https://github.com/almeidx/versi/blob/main/LICENSE"
            ),
        ]
        .spacing(8),
        Space::new().height(10),
        text("Backends").size(13),
        text(format!("Active backend: {}", state.backend_name))
            .size(12)
            .color(crate::theme::tokens::TEXT_MUTED),
        text(format!("Detected in this session: {backend_summary}"))
            .size(12)
            .color(crate::theme::tokens::TEXT_MUTED),
        text("Versi is backend-agnostic and currently supports fnm, nvm, and Volta.")
            .size(12)
            .color(crate::theme::tokens::TEXT_MUTED),
        row![
            link_button("fnm", "https://github.com/Schniz/fnm"),
            link_button("nvm", "https://github.com/nvm-sh/nvm"),
            link_button("Volta", "https://github.com/volta-cli/volta"),
        ]
        .spacing(8),
        Space::new().height(10),
        text("License").size(13),
        text("Versi is distributed under the GPL-3.0-only license.")
            .size(12)
            .color(crate::theme::tokens::TEXT_MUTED),
    ]
    .spacing(6)
    .width(Length::Fill);

    let centered_content = container(
        container(content)
            .style(styles::card_container)
            .padding([20, 24])
            .width(Length::Fill)
            .max_width(760),
    )
    .width(Length::Fill)
    .center_x(Length::Fill);

    let centered_body = container(centered_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill);

    let page_content = column![header, Space::new().height(12), centered_body]
        .spacing(0)
        .padding(content_padding)
        .width(Length::Fill)
        .height(Length::Fill);

    container(page_content)
        .padding(iced::Padding::new(0.0).right(crate::theme::tokens::SCROLL_CONTENT_RIGHT_INSET))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn link_button<'a>(label: &'a str, url: &'a str) -> iced::widget::Button<'a, Message> {
    button(
        row![text(label).size(12), icon::arrow_up_right(12.0),]
            .spacing(4)
            .align_y(Alignment::Center),
    )
    .on_press(Message::OpenLink(url.to_string()))
    .style(styles::secondary_button)
    .padding([6, 12])
}

fn detected_backends_summary(backends: &[BackendKind]) -> String {
    if backends.is_empty() {
        return "none".to_string();
    }

    let mut names: Vec<&str> = backends.iter().map(|backend| backend.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    names.join(", ")
}

#[cfg(test)]
mod tests {
    use super::detected_backends_summary;
    use crate::backend_kind::BackendKind;

    #[test]
    fn detected_backends_summary_reports_none_for_empty_input() {
        assert_eq!(detected_backends_summary(&[]), "none");
    }

    #[test]
    fn detected_backends_summary_sorts_and_deduplicates_names() {
        let names = detected_backends_summary(&[
            BackendKind::Nvm,
            BackendKind::Volta,
            BackendKind::Fnm,
            BackendKind::Nvm,
        ]);

        assert_eq!(names, "fnm, nvm, volta");
    }
}
