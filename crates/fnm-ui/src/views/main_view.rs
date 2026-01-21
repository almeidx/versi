use iced::widget::{button, column, container, mouse_area, row, text, text_input, Space};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::settings::AppSettings;
use crate::state::{MainState, Modal, Operation, SettingsModalState, ShellVerificationStatus};
use crate::theme::styles;
use crate::widgets::{install_modal, toast_container, version_list};

pub fn view<'a>(state: &'a MainState, _settings: &'a AppSettings) -> Element<'a, Message> {
    let header = header_view(state);
    let search_bar = search_bar_view(state);
    let version_list = version_list::view(
        state.active_environment(),
        &state.search_query,
        &state.available_versions.versions,
    );
    let operation_status = operation_status_view(state);

    let main_content = column![header, search_bar, version_list, operation_status,]
        .spacing(20)
        .padding(32);

    let with_modal: Element<Message> = if let Some(modal) = &state.modal {
        modal_overlay(main_content.into(), modal, state)
    } else {
        main_content.into()
    };

    toast_container::view(with_modal, &state.toasts)
}

fn header_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    let env = state.active_environment();

    let title_section = column![
        text("Node Versions").size(28),
        match &env.default_version {
            Some(v) => text(format!("Default: {}", v)).size(13),
            None => text("No default set").size(13),
        },
    ]
    .spacing(2);

    let mut button_row = row![
        button(text("Install").size(13))
            .on_press(Message::OpenInstallModal)
            .style(styles::primary_button)
            .padding([8, 16]),
        button(text("Refresh").size(13))
            .on_press(Message::RefreshEnvironment)
            .style(styles::secondary_button)
            .padding([8, 16]),
        button(text("Settings").size(13))
            .on_press(Message::OpenSettings)
            .style(styles::ghost_button)
            .padding([8, 12]),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if let Some(update) = &state.app_update {
        button_row = button_row.push(
            button(
                container(text(format!("v{} available", update.latest_version)).size(11))
                    .padding([2, 8]),
            )
            .on_press(Message::OpenAppUpdate)
            .style(styles::app_update_button)
            .padding(0),
        );
    }

    row![title_section, Space::new().width(Length::Fill), button_row,]
        .align_y(Alignment::Center)
        .into()
}

fn search_bar_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    text_input("Search versions...", &state.search_query)
        .on_input(Message::SearchChanged)
        .padding(14)
        .size(14)
        .style(styles::search_input)
        .into()
}

fn operation_status_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    if let Some(op) = &state.current_operation {
        match op {
            Operation::Install { version, progress } => {
                let phase_text = match progress.phase {
                    fnm_core::InstallPhase::Starting => "Preparing...",
                    fnm_core::InstallPhase::Downloading => "Downloading...",
                    fnm_core::InstallPhase::Extracting => "Extracting...",
                    fnm_core::InstallPhase::Installing => "Installing...",
                    fnm_core::InstallPhase::Complete => "Complete!",
                    fnm_core::InstallPhase::Failed => "Failed",
                };

                container(
                    column![
                        text(format!("Installing Node {}", version)).size(14),
                        text(phase_text).size(12),
                    ]
                    .spacing(8)
                    .padding(20),
                )
                .style(styles::card_container)
                .into()
            }
            Operation::Uninstall { version } => container(
                row![text(format!("Removing Node {}...", version)).size(14),]
                    .spacing(8)
                    .padding(20),
            )
            .style(styles::card_container)
            .into(),
            Operation::SetDefault { version, .. } => container(
                row![text(format!("Setting default to Node {}...", version)).size(14),]
                    .spacing(8)
                    .padding(20),
            )
            .style(styles::card_container)
            .into(),
        }
    } else {
        Space::new().into()
    }
}

fn modal_overlay<'a>(
    content: Element<'a, Message>,
    modal: &'a Modal,
    state: &'a MainState,
) -> Element<'a, Message> {
    let modal_content: Element<Message> = match modal {
        Modal::Install(install_state) => install_modal::view(install_state, state),
        Modal::Settings(settings_state) => settings_modal_view(settings_state),
        Modal::ConfirmUninstall { version } => confirm_uninstall_view(version),
    };

    let backdrop = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .style(|_theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.4,
                })),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::CloseModal);

    let modal_container = mouse_area(
        container(modal_content)
            .style(styles::card_container)
            .padding(28)
            .max_width(480),
    )
    .on_press(Message::NoOp);

    let modal_layer = container(modal_container)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill);

    iced::widget::stack![content, backdrop, modal_layer].into()
}

fn settings_modal_view<'a>(settings: &'a SettingsModalState) -> Element<'a, Message> {
    let mut content = column![
        row![
            text("Settings").size(20),
            Space::new().width(Length::Fill),
            button(text("Done").size(13))
                .on_press(Message::CloseSettings)
                .style(styles::primary_button)
                .padding([6, 14]),
        ]
        .align_y(Alignment::Center),
        Space::new().height(24),
        text("Appearance").size(13),
        Space::new().height(8),
        row![
            button(text("System").size(13))
                .on_press(Message::ThemeChanged(crate::settings::ThemeSetting::System))
                .style(styles::secondary_button)
                .padding([10, 16]),
            button(text("Light").size(13))
                .on_press(Message::ThemeChanged(crate::settings::ThemeSetting::Light))
                .style(styles::secondary_button)
                .padding([10, 16]),
            button(text("Dark").size(13))
                .on_press(Message::ThemeChanged(crate::settings::ThemeSetting::Dark))
                .style(styles::secondary_button)
                .padding([10, 16]),
        ]
        .spacing(8),
        Space::new().height(24),
        text("Shell Setup").size(13),
        Space::new().height(8),
    ]
    .spacing(4)
    .width(Length::Fill);

    if settings.checking_shells {
        content = content.push(text("Checking shell configuration...").size(12));
    } else if settings.shell_statuses.is_empty() {
        content = content.push(text("No shells detected").size(12));
    } else {
        for shell in &settings.shell_statuses {
            let status_text = match &shell.status {
                ShellVerificationStatus::Unknown => "Unknown",
                ShellVerificationStatus::Configured => "Configured ✓",
                ShellVerificationStatus::NotConfigured => "Not configured",
                ShellVerificationStatus::FunctionalButNotInConfig => "Working (not in config)",
                ShellVerificationStatus::Error(_) => "Error",
            };

            let is_configured = matches!(
                shell.status,
                ShellVerificationStatus::Configured
                    | ShellVerificationStatus::FunctionalButNotInConfig
            );

            let shell_row = if shell.configuring {
                row![
                    text(&shell.shell_name).size(13).width(Length::Fixed(100.0)),
                    text("Configuring...").size(12),
                ]
            } else if is_configured {
                row![
                    text(&shell.shell_name).size(13).width(Length::Fixed(100.0)),
                    text(status_text)
                        .size(12)
                        .color(iced::Color::from_rgb8(52, 199, 89)),
                ]
            } else {
                let shell_type = shell.shell_type.clone();
                row![
                    text(&shell.shell_name).size(13).width(Length::Fixed(100.0)),
                    text(status_text)
                        .size(12)
                        .color(iced::Color::from_rgb8(255, 149, 0)),
                    Space::new().width(Length::Fill),
                    button(text("Configure").size(11))
                        .on_press(Message::ConfigureShell(shell_type))
                        .style(styles::secondary_button)
                        .padding([4, 10]),
                ]
            };

            content = content.push(shell_row.spacing(8).align_y(Alignment::Center));
        }
    }

    content.into()
}

fn confirm_uninstall_view<'a>(version: &'a str) -> Element<'a, Message> {
    column![
        text(format!("Remove Node {}?", version)).size(20),
        Space::new().height(12),
        text("This version will be uninstalled from your system.").size(14),
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelUninstall)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Remove").size(13))
                .on_press(Message::ConfirmUninstall(version.to_string()))
                .style(styles::danger_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}
