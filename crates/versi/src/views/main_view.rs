use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, text, text_input, toggler,
};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::settings::{AppSettings, TrayBehavior};
use crate::state::{
    MainState, Modal, Operation, QueuedOperation, SettingsModalState, ShellVerificationStatus,
};
use crate::theme::{is_system_dark, styles};
use crate::widgets::{toast_container, version_list};

pub fn view<'a>(state: &'a MainState, settings: &'a AppSettings) -> Element<'a, Message> {
    let header = header_view(state);
    let search_bar = search_bar_view(state);
    let version_list = version_list::view(
        state.active_environment(),
        &state.search_query,
        &state.available_versions.versions,
        state.available_versions.schedule.as_ref(),
        &state.operation_queue,
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

    let subtitle = match (&env.default_version, &env.fnm_version) {
        (Some(v), Some(fnm_v)) => format!("Default: {} · fnm {}", v, fnm_v),
        (Some(v), None) => format!("Default: {}", v),
        (None, Some(fnm_v)) => format!("fnm {}", fnm_v),
        (None, None) => "No default set".to_string(),
    };

    let title_section =
        column![text("Node Versions").size(28), text(subtitle).size(13),].spacing(2);

    let mut button_row = row![
        button(text("Refresh").size(13))
            .on_press(Message::RefreshEnvironment)
            .style(styles::secondary_button)
            .padding([8, 16]),
        button(text("Update All").size(13))
            .on_press(Message::RequestBulkUpdateMajors)
            .style(styles::secondary_button)
            .padding([8, 12]),
        button(text("Clean EOL").size(13))
            .on_press(Message::RequestBulkUninstallEOL)
            .style(styles::ghost_button)
            .padding([8, 12]),
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
    text_input(
        "Search or install versions (e.g., '22', 'lts')...",
        &state.search_query,
    )
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

            if !env.available {
                let label = if let Some(reason) = &env.error {
                    format!("{} ({})", env.name, reason)
                } else {
                    format!("{} (Unavailable)", env.name)
                };
                return button(text(label).size(13))
                    .style(styles::disabled_tab_button)
                    .padding([8, 16])
                    .into();
            }

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
    let queue = &state.operation_queue;

    if queue.current.is_none() && queue.pending.is_empty() {
        return None;
    }

    let mut content = column![].spacing(8);

    if let Some(op) = &queue.current {
        content = content.push(current_operation_view(op));
    }

    if !queue.pending.is_empty() {
        content = content.push(text("Queued").size(11));
        for queued in &queue.pending {
            content = content.push(queued_operation_view(queued));
        }
    }

    Some(
        container(content.padding(16))
            .style(styles::card_container)
            .max_width(320)
            .into(),
    )
}

fn current_operation_view(op: &Operation) -> Element<'_, Message> {
    match op {
        Operation::Install { version, progress } => {
            let phase_text = match progress.phase {
                versi_core::InstallPhase::Starting => "Preparing...",
                versi_core::InstallPhase::Downloading => "Downloading...",
                versi_core::InstallPhase::Extracting => "Extracting...",
                versi_core::InstallPhase::Installing => "Installing...",
                versi_core::InstallPhase::Complete => "Complete!",
                versi_core::InstallPhase::Failed => "Failed",
            };

            column![
                text(format!("Installing Node {}", version)).size(14),
                text(phase_text).size(12),
            ]
            .spacing(4)
            .into()
        }
        Operation::Uninstall { version } => text(format!("Removing Node {}...", version))
            .size(14)
            .into(),
        Operation::SetDefault { version } => text(format!("Setting default to {}...", version))
            .size(14)
            .into(),
    }
}

fn queued_operation_view(queued: &QueuedOperation) -> Element<'_, Message> {
    row![
        text(queued.request.description()).size(12),
        Space::new().width(Length::Fill),
        button(text("×").size(12))
            .on_press(Message::CancelQueuedOperation(queued.id))
            .style(styles::ghost_button)
            .padding([2, 6]),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn modal_overlay<'a>(
    content: Element<'a, Message>,
    modal: &'a Modal,
    state: &'a MainState,
    settings: &'a AppSettings,
) -> Element<'a, Message> {
    let modal_content: Element<Message> = match modal {
        Modal::Settings(settings_state) => settings_modal_view(settings_state, settings, state),
        Modal::ConfirmUninstall {
            version,
            is_default,
        } => confirm_uninstall_view(version, *is_default),
        Modal::ConfirmBulkUpdateMajors { versions } => confirm_bulk_update_view(versions),
        Modal::ConfirmBulkUninstallEOL { versions } => confirm_bulk_uninstall_eol_view(versions),
        Modal::ConfirmBulkUninstallMajor { major, versions } => {
            confirm_bulk_uninstall_major_view(*major, versions)
        }
        Modal::ConfirmBulkUninstallMajorExceptLatest {
            major,
            versions,
            keeping,
        } => confirm_bulk_uninstall_major_except_latest_view(*major, versions, keeping),
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
    let header = row![
        text("Settings").size(20),
        Space::new().width(Length::Fill),
        button(text("Done").size(13))
            .on_press(Message::CloseSettings)
            .style(styles::primary_button)
            .padding([6, 14]),
    ]
    .align_y(Alignment::Center);

    let mut content = column![
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
        text("System Tray").size(13),
        Space::new().height(8),
        row![
            button(text("When Open").size(13))
                .on_press(Message::TrayBehaviorChanged(TrayBehavior::WhenWindowOpen))
                .style(if settings.tray_behavior == TrayBehavior::WhenWindowOpen {
                    styles::primary_button
                } else {
                    styles::secondary_button
                })
                .padding([10, 16]),
            button(text("Always").size(13))
                .on_press(Message::TrayBehaviorChanged(TrayBehavior::AlwaysRunning))
                .style(if settings.tray_behavior == TrayBehavior::AlwaysRunning {
                    styles::primary_button
                } else {
                    styles::secondary_button
                })
                .padding([10, 16]),
            button(text("Disabled").size(13))
                .on_press(Message::TrayBehaviorChanged(TrayBehavior::Disabled))
                .style(if settings.tray_behavior == TrayBehavior::Disabled {
                    styles::primary_button
                } else {
                    styles::secondary_button
                })
                .padding([10, 16]),
        ]
        .spacing(8),
        row![
            toggler(settings.start_minimized)
                .on_toggle(Message::StartMinimizedToggled)
                .size(18),
            text("Start minimized to tray").size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text("\"Always\" keeps the app running in the tray when closed")
            .size(11)
            .color(iced::Color::from_rgb8(142, 142, 147)),
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
    let log_size_text = match modal_state.log_file_size {
        Some(0) => "empty".to_string(),
        Some(size) if size < 1024 => format!("{} B", size),
        Some(size) if size < 1024 * 1024 => format!("{:.1} KB", size as f64 / 1024.0),
        Some(size) => format!("{:.1} MB", size as f64 / (1024.0 * 1024.0)),
        None => "not found".to_string(),
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
            text(format!(" ({})", log_size_text))
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        ]
        .align_y(Alignment::Center),
    );
    content = content.push(Space::new().height(8));
    content = content.push(
        row![
            button(text("Show in Folder").size(11))
                .on_press(Message::RevealLogFile)
                .style(styles::secondary_button)
                .padding([4, 10]),
            button(text("Clear Log").size(11))
                .on_press(Message::ClearLogFile)
                .style(styles::secondary_button)
                .padding([4, 10]),
        ]
        .spacing(8),
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

    column![
        header,
        Space::new().height(24),
        scrollable(content.padding(iced::Padding::default().right(12))).height(Length::Fill),
    ]
    .spacing(0)
    .width(Length::Fill)
    .into()
}

fn confirm_uninstall_view<'a>(version: &'a str, is_default: bool) -> Element<'a, Message> {
    let mut content = column![
        text(format!("Remove Node {}?", version)).size(20),
        Space::new().height(12),
    ]
    .spacing(4);

    if is_default {
        content = content
            .push(
                text("Warning: This is your default Node.js version.")
                    .size(14)
                    .color(styles::WARNING_COLOR),
            )
            .push(Space::new().height(4))
            .push(
                text("You will need to set a new default after uninstalling.")
                    .size(14)
                    .color(styles::WARNING_COLOR),
            );
    } else {
        content = content.push(text("This version will be uninstalled from your system.").size(14));
    }

    content = content.push(Space::new().height(24)).push(
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
    );

    content.width(Length::Fill).into()
}

fn confirm_bulk_update_view(versions: &[(String, String)]) -> Element<'_, Message> {
    let mut version_list = column![].spacing(4);

    for (from, to) in versions.iter().take(10) {
        version_list = version_list.push(
            text(format!("{} → {}", from, to))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > 10 {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - 10))
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    column![
        text("Update All Versions?").size(20),
        Space::new().height(12),
        text(format!(
            "This will install {} newer version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_list,
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelBulkOperation)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Update All").size(13))
                .on_press(Message::ConfirmBulkUpdateMajors)
                .style(styles::primary_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn confirm_bulk_uninstall_eol_view(versions: &[String]) -> Element<'_, Message> {
    let mut version_list = column![].spacing(4);

    for version in versions.iter().take(10) {
        version_list = version_list.push(
            text(format!("Node {}", version))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > 10 {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - 10))
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    column![
        text("Remove All EOL Versions?").size(20),
        Space::new().height(12),
        text(format!(
            "This will uninstall {} end-of-life version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_list,
        Space::new().height(8),
        text("These versions no longer receive security updates.")
            .size(12)
            .color(iced::Color::from_rgb8(255, 149, 0)),
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelBulkOperation)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Remove All").size(13))
                .on_press(Message::ConfirmBulkUninstallEOL)
                .style(styles::danger_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn confirm_bulk_uninstall_major_view(major: u32, versions: &[String]) -> Element<'_, Message> {
    let mut version_list = column![].spacing(4);

    for version in versions.iter().take(10) {
        version_list = version_list.push(
            text(format!("Node {}", version))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > 10 {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - 10))
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    column![
        text(format!("Remove All Node {}.x Versions?", major)).size(20),
        Space::new().height(12),
        text(format!(
            "This will uninstall {} version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_list,
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelBulkOperation)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Remove All").size(13))
                .on_press(Message::ConfirmBulkUninstallMajor { major })
                .style(styles::danger_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn confirm_bulk_uninstall_major_except_latest_view<'a>(
    major: u32,
    versions: &'a [String],
    keeping: &'a str,
) -> Element<'a, Message> {
    let mut version_list = column![].spacing(4);

    for version in versions.iter().take(10) {
        version_list = version_list.push(
            text(format!("Node {}", version))
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    if versions.len() > 10 {
        version_list = version_list.push(
            text(format!("...and {} more", versions.len() - 10))
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    column![
        text(format!("Clean Up Node {}.x Versions?", major)).size(20),
        Space::new().height(12),
        text(format!(
            "This will uninstall {} older version(s):",
            versions.len()
        ))
        .size(14),
        Space::new().height(8),
        version_list,
        Space::new().height(8),
        text(format!("Node {} will be kept.", keeping))
            .size(12)
            .color(iced::Color::from_rgb8(52, 199, 89)),
        Space::new().height(24),
        row![
            button(text("Cancel").size(13))
                .on_press(Message::CancelBulkOperation)
                .style(styles::secondary_button)
                .padding([10, 20]),
            Space::new().width(Length::Fill),
            button(text("Remove Older").size(13))
                .on_press(Message::ConfirmBulkUninstallMajorExceptLatest { major })
                .style(styles::danger_button)
                .padding([10, 20]),
        ]
        .spacing(16),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}
