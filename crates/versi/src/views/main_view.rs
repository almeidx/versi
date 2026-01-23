use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, toggler, Space,
};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::settings::AppSettings;
use crate::state::{MainState, Modal, Operation, SettingsModalState, ShellVerificationStatus};
use crate::theme::{is_system_dark, styles};
use crate::widgets::{install_modal, toast_container, version_list};

pub fn view<'a>(state: &'a MainState, settings: &'a AppSettings) -> Element<'a, Message> {
    let header = header_view(state);
    let search_bar = search_bar_view(state);
    let version_list = version_list::view(
        state.active_environment(),
        &state.search_query,
        &state.available_versions.versions,
        state.available_versions.schedule.as_ref(),
    );

    let mut main_column = column![].spacing(0);

    if let Some(tabs) = environment_tabs_view(state) {
        main_column = main_column.push(
            container(tabs).padding(iced::Padding::new(0.0).top(16.0).left(32.0).right(32.0)),
        );
    }

    let main_content = column![header, search_bar, version_list]
        .spacing(20)
        .padding(32);

    main_column = main_column.push(main_content);

    let with_modal: Element<Message> = if let Some(modal) = &state.modal {
        modal_overlay(main_column.into(), modal, state, settings)
    } else {
        main_column.into()
    };

    let with_toasts = toast_container::view(with_modal, &state.toasts);

    if let Some(operation) = operation_status_view(state) {
        let operation_overlay = container(operation)
            .padding(16)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Bottom)
            .width(Length::Fill)
            .height(Length::Fill);

        iced::widget::stack![with_toasts, operation_overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        with_toasts
    }
}

fn header_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    let env = state.active_environment();

    let subtitle = match (&env.default_version, &state.fnm_version) {
        (Some(v), Some(fnm_v)) => format!("Default: {} · fnm {}", v, fnm_v),
        (Some(v), None) => format!("Default: {}", v),
        (None, Some(fnm_v)) => format!("fnm {}", fnm_v),
        (None, None) => "No default set".to_string(),
    };

    let title_section =
        column![text("Node Versions").size(28), text(subtitle).size(13),].spacing(2);

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

    if let Some(update) = &state.fnm_update {
        button_row = button_row.push(
            button(
                container(text(format!("fnm {} available", update.latest_version)).size(11))
                    .padding([2, 8]),
            )
            .on_press(Message::OpenFnmUpdate)
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

fn environment_tabs_view<'a>(state: &'a MainState) -> Option<Element<'a, Message>> {
    if state.environments.len() <= 1 {
        return None;
    }

    let tabs: Vec<_> = state
        .environments
        .iter()
        .enumerate()
        .map(|(idx, env)| {
            let is_active = idx == state.active_environment_idx;
            let style = if is_active {
                styles::active_tab_button
            } else {
                styles::inactive_tab_button
            };

            button(text(&env.name).size(13))
                .on_press(Message::EnvironmentSelected(idx))
                .style(style)
                .padding([8, 16])
                .into()
        })
        .collect();

    Some(row(tabs).spacing(4).into())
}

fn operation_status_view<'a>(state: &'a MainState) -> Option<Element<'a, Message>> {
    let op = state.current_operation.as_ref()?;

    let element = match op {
        Operation::Install { version, progress } => {
            let phase_text = match progress.phase {
                versi_core::InstallPhase::Starting => "Preparing...",
                versi_core::InstallPhase::Downloading => "Downloading...",
                versi_core::InstallPhase::Extracting => "Extracting...",
                versi_core::InstallPhase::Installing => "Installing...",
                versi_core::InstallPhase::Complete => "Complete!",
                versi_core::InstallPhase::Failed => "Failed",
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
    };

    Some(element)
}

fn modal_overlay<'a>(
    content: Element<'a, Message>,
    modal: &'a Modal,
    state: &'a MainState,
    settings: &'a AppSettings,
) -> Element<'a, Message> {
    let modal_content: Element<Message> = match modal {
        Modal::Install(install_state) => install_modal::view(install_state, state),
        Modal::Settings(settings_state) => settings_modal_view(settings_state, settings, state),
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

fn settings_modal_view<'a>(
    modal_state: &'a SettingsModalState,
    settings: &'a AppSettings,
    _state: &'a MainState,
) -> Element<'a, Message> {
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
            button(
                text(if is_system_dark() {
                    "System (Dark)"
                } else {
                    "System (Light)"
                })
                .size(13),
            )
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
        text("Shell Options").size(13),
        Space::new().height(8),
        row![
            toggler(settings.shell_options.use_on_cd)
                .on_toggle(Message::ShellOptionUseOnCdToggled)
                .size(18),
            text("Auto-switch on cd").size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            toggler(settings.shell_options.resolve_engines)
                .on_toggle(Message::ShellOptionResolveEnginesToggled)
                .size(18),
            text("Resolve engines from package.json").size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            toggler(settings.shell_options.corepack_enabled)
                .on_toggle(Message::ShellOptionCorepackEnabledToggled)
                .size(18),
            text("Enable corepack").size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text("Options for new shell configurations")
            .size(11)
            .color(iced::Color::from_rgb8(142, 142, 147)),
    ]
    .spacing(4)
    .width(Length::Fill);

    content = content.push(Space::new().height(24));
    content = content.push(text("Shell Setup").size(13));
    content = content.push(Space::new().height(8));

    if modal_state.checking_shells {
        content = content.push(text("Checking shell configuration...").size(12));
    } else if modal_state.shell_statuses.is_empty() {
        content = content.push(text("No shells detected").size(12));
    } else {
        for shell in &modal_state.shell_statuses {
            let status_text = match &shell.status {
                ShellVerificationStatus::Unknown => "Unknown",
                ShellVerificationStatus::Configured => "Configured ✓",
                ShellVerificationStatus::NotConfigured => "Not configured",
                ShellVerificationStatus::NoConfigFile => "No config file",
                ShellVerificationStatus::FunctionalButNotInConfig => "Working (not in config)",
                ShellVerificationStatus::Error(_) => "Error",
            };

            let is_configured = matches!(
                shell.status,
                ShellVerificationStatus::Configured
                    | ShellVerificationStatus::FunctionalButNotInConfig
            );

            let has_no_config_file = matches!(shell.status, ShellVerificationStatus::NoConfigFile);

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
            } else if has_no_config_file {
                row![
                    text(&shell.shell_name).size(13).width(Length::Fixed(100.0)),
                    text(status_text)
                        .size(12)
                        .color(iced::Color::from_rgb8(142, 142, 147)),
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

    content = content.push(Space::new().height(24));
    content = content.push(text("Advanced").size(13));
    content = content.push(Space::new().height(8));
    content = content.push(
        row![
            toggler(settings.debug_logging)
                .on_toggle(Message::DebugLoggingToggled)
                .size(18),
            text("Debug logging").size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    let log_path = {
        let paths = versi_platform::AppPaths::new();
        paths.log_file().to_string_lossy().to_string()
    };
    content = content.push(
        row![
            text("Log file: ")
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
            button(text(log_path.clone()).size(11))
                .on_press(Message::CopyToClipboard(log_path))
                .style(styles::link_button)
                .padding(0),
        ]
        .align_y(Alignment::Center),
    );
    content = content.push(
        text("Restart required to enable/disable or clear log file")
            .size(11)
            .color(iced::Color::from_rgb8(142, 142, 147)),
    );

    content = content.push(Space::new().height(24));
    content = content.push(text("About").size(13));
    content = content.push(Space::new().height(8));
    content = content.push(text(format!("Versi v{}", env!("CARGO_PKG_VERSION"))).size(14));
    content = content.push(Space::new().height(4));
    content = content.push(
        text("A native GUI for fnm (Fast Node Manager)")
            .size(12)
            .color(iced::Color::from_rgb8(142, 142, 147)),
    );
    content = content.push(Space::new().height(12));
    content = content.push(
        row![
            button(text("GitHub").size(12))
                .on_press(Message::OpenLink(
                    "https://github.com/almeidx/versi".to_string()
                ))
                .style(styles::secondary_button)
                .padding([6, 12]),
            button(text("fnm").size(12))
                .on_press(Message::OpenLink(
                    "https://github.com/Schniz/fnm".to_string()
                ))
                .style(styles::secondary_button)
                .padding([6, 12]),
        ]
        .spacing(8),
    );

    scrollable(content).height(Length::Shrink).into()
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
