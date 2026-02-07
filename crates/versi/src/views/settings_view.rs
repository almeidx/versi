use iced::widget::{Space, button, column, container, row, scrollable, text, toggler, tooltip};
use iced::{Alignment, Element, Length};

use crate::icon;
use crate::message::Message;
use crate::settings::{AppSettings, ThemeSetting, TrayBehavior};
use crate::state::{MainState, SettingsModalState, ShellVerificationStatus};
use crate::theme::styles;
use crate::widgets::helpers::nav_icons;

pub fn view<'a>(
    settings_state: &'a SettingsModalState,
    settings: &'a AppSettings,
    state: &'a MainState,
    has_tabs: bool,
    is_system_dark: bool,
) -> Element<'a, Message> {
    let header = row![
        text("Settings").size(14),
        Space::new().width(Length::Fill),
        nav_icons(&state.view, state.refresh_rotation),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let capabilities = state.backend.capabilities();
    let shell_opts = settings.shell_options_for(state.backend_name);

    let mut content = column![
        text("Appearance").size(14),
        Space::new().height(8),
        row![
            button(
                text(if is_system_dark {
                    "System (Dark)"
                } else {
                    "System (Light)"
                })
                .size(13),
            )
            .on_press(Message::ThemeChanged(ThemeSetting::System))
            .style(if settings.theme == ThemeSetting::System {
                styles::primary_button
            } else {
                styles::secondary_button
            })
            .padding([10, 16]),
            button(text("Light").size(13))
                .on_press(Message::ThemeChanged(ThemeSetting::Light))
                .style(if settings.theme == ThemeSetting::Light {
                    styles::primary_button
                } else {
                    styles::secondary_button
                })
                .padding([10, 16]),
            button(text("Dark").size(13))
                .on_press(Message::ThemeChanged(ThemeSetting::Dark))
                .style(if settings.theme == ThemeSetting::Dark {
                    styles::primary_button
                } else {
                    styles::secondary_button
                })
                .padding([10, 16]),
        ]
        .spacing(8),
        Space::new().height(28),
        text("Preferred Engine").size(14),
        Space::new().height(8),
        engine_selector(settings, state),
        text(format!("Currently using: {}", state.backend_name))
            .size(11)
            .color(iced::Color::from_rgb8(142, 142, 147)),
        text("Each environment uses whichever engine is available")
            .size(11)
            .color(iced::Color::from_rgb8(142, 142, 147)),
        Space::new().height(28),
        text("System Tray").size(14),
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
        Space::new().height(8),
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
        Space::new().height(28),
        text("Shell Options").size(14),
        Space::new().height(8),
    ]
    .spacing(4)
    .width(Length::Fill);

    if capabilities.supports_auto_switch {
        content = content.push(
            row![
                toggler(shell_opts.use_on_cd)
                    .on_toggle(Message::ShellOptionUseOnCdToggled)
                    .size(18),
                text("Auto-switch on cd").size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    if capabilities.supports_resolve_engines {
        content = content.push(
            row![
                toggler(shell_opts.resolve_engines)
                    .on_toggle(Message::ShellOptionResolveEnginesToggled)
                    .size(18),
                text("Resolve engines from package.json").size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    if capabilities.supports_corepack {
        content = content.push(
            row![
                toggler(shell_opts.corepack_enabled)
                    .on_toggle(Message::ShellOptionCorepackEnabledToggled)
                    .size(18),
                text("Enable corepack").size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    if !capabilities.supports_auto_switch
        && !capabilities.supports_resolve_engines
        && !capabilities.supports_corepack
    {
        content = content.push(
            text("No shell options available for this engine")
                .size(12)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    } else {
        content = content.push(
            text("Options for new shell configurations")
                .size(11)
                .color(iced::Color::from_rgb8(142, 142, 147)),
        );
    }

    content = content.push(Space::new().height(28));
    content = content.push(text("Shell Setup").size(14));
    content = content.push(Space::new().height(8));

    if settings_state.checking_shells {
        content = content.push(text("Checking shell configuration...").size(12));
    } else if settings_state.shell_statuses.is_empty() {
        content = content.push(text("No shells detected").size(12));
    } else {
        for shell in &settings_state.shell_statuses {
            let is_configured_check = matches!(shell.status, ShellVerificationStatus::Configured);

            let status_text = match &shell.status {
                ShellVerificationStatus::Configured => "Configured",
                ShellVerificationStatus::NotConfigured => "Not configured",
                ShellVerificationStatus::NoConfigFile => "No config file",
                ShellVerificationStatus::FunctionalButNotInConfig => "Working (not in config)",
                ShellVerificationStatus::Error => "Error",
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
                let mut r = row![
                    text(&shell.shell_name).size(13).width(Length::Fixed(100.0)),
                    text(status_text)
                        .size(12)
                        .color(iced::Color::from_rgb8(52, 199, 89)),
                ]
                .spacing(8)
                .align_y(Alignment::Center);
                if is_configured_check {
                    let check_icon: Element<'_, Message> = icon::check(12.0)
                        .style(|_theme: &iced::Theme, _status| iced::widget::svg::Style {
                            color: Some(iced::Color::from_rgb8(52, 199, 89)),
                        })
                        .into();
                    r = r.push(check_icon);
                }
                r
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

    content = content.push(Space::new().height(28));
    content = content.push(text("Settings Data").size(14));
    content = content.push(Space::new().height(8));
    content = content.push(
        row![
            button(text("Export").size(11))
                .on_press(Message::ExportSettings)
                .style(styles::secondary_button)
                .padding([4, 10]),
            button(text("Import").size(11))
                .on_press(Message::ImportSettings)
                .style(styles::secondary_button)
                .padding([4, 10]),
            button(text("Show in Folder").size(11))
                .on_press(Message::RevealSettingsFile)
                .style(styles::secondary_button)
                .padding([4, 10]),
        ]
        .spacing(8),
    );
    content = content.push(
        text("Export or import preferences, or edit the config file directly")
            .size(11)
            .color(iced::Color::from_rgb8(142, 142, 147)),
    );

    content = content.push(Space::new().height(28));
    content = content.push(text("Advanced").size(14));
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
    let log_path = versi_platform::AppPaths::new()
        .map(|p| p.log_file().to_string_lossy().to_string())
        .unwrap_or_default();
    let log_size_text = match settings_state.log_file_size {
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
    column![
        container(header).padding(iced::Padding::new(0.0).right(24.0)),
        Space::new().height(12),
        scrollable(content.padding(iced::Padding::default().right(24.0))).height(Length::Fill),
    ]
    .spacing(0)
    .padding(if has_tabs {
        iced::Padding::new(24.0).right(0.0)
    } else {
        iced::Padding::new(24.0).top(12.0).right(0.0)
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn engine_button<'a>(
    name: &'static str,
    is_selected: bool,
    is_detected: bool,
) -> Element<'a, Message> {
    let btn = button(text(name).size(13))
        .style(if is_selected {
            styles::primary_button
        } else {
            styles::secondary_button
        })
        .padding([10, 16]);

    if is_detected {
        btn.on_press(Message::PreferredBackendChanged(name.to_string()))
            .into()
    } else {
        tooltip(
            btn,
            container(text(format!("{} is not installed", name)).size(12))
                .padding([4, 8])
                .style(styles::tooltip_container),
            tooltip::Position::Bottom,
        )
        .gap(4.0)
        .into()
    }
}

fn engine_selector<'a>(settings: &'a AppSettings, state: &'a MainState) -> Element<'a, Message> {
    let preferred = settings.preferred_backend.as_deref().unwrap_or("fnm");
    let fnm_detected = state.detected_backends.contains(&"fnm");
    let nvm_detected = state.detected_backends.contains(&"nvm");

    row![
        engine_button("fnm", preferred == "fnm", fnm_detected),
        engine_button("nvm", preferred == "nvm", nvm_detected),
    ]
    .spacing(8)
    .into()
}
