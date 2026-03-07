use iced::widget::{Space, button, column, container, row, scrollable, text, toggler, tooltip};
use iced::{Alignment, Element, Length};

use crate::backend_kind::BackendKind;
use crate::format::format_tenths;
use crate::icon;
use crate::message::Message;
use crate::settings::{AppSettings, AppUpdateBehavior, ThemeSetting, TrayBehavior};
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
    let content_padding = super::content_padding(has_tabs).right(crate::theme::tokens::INSET_RIGHT);
    let top_content_right_inset =
        content_padding.right + crate::theme::tokens::SCROLL_CONTENT_RIGHT_INSET;
    let capabilities = state.backend.capabilities();
    let shell_opts = settings.shell_options_for(state.backend_name);

    let content = column![
        appearance_section(settings, is_system_dark),
        preferred_engine_section(settings, state),
        tray_section(settings),
        update_behavior_section(settings),
        shell_options_section(capabilities, shell_opts),
        shell_setup_section(settings_state),
        settings_data_section(),
        advanced_section(settings_state, settings),
    ]
    .spacing(4)
    .width(Length::Fill);

    let header = container(settings_header(state))
        .padding(iced::Padding {
            top: content_padding.top,
            right: top_content_right_inset,
            bottom: 12.0,
            left: content_padding.left,
        })
        .style(styles::page_background_overlay)
        .width(Length::Fill);

    let scroll_content = column![content]
        .spacing(0)
        .padding(content_padding)
        .width(Length::Fill);

    let main_scrollable = scrollable(scroll_content)
        .direction(iced::widget::scrollable::Direction::Vertical(
            styles::overlay_scrollbar(),
        ))
        .style(styles::overlay_scrollable)
        .width(Length::Fill)
        .height(Length::Fill);

    column![header, main_scrollable]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn settings_header(state: &MainState) -> iced::widget::Row<'_, Message> {
    row![
        text("Settings").size(14),
        Space::new().width(Length::Fill),
        nav_icons(&state.view, state.refresh_rotation),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
}

fn appearance_section(settings: &AppSettings, is_system_dark: bool) -> Element<'_, Message> {
    let system_label = if is_system_dark {
        "System (Dark)"
    } else {
        "System (Light)"
    };
    column![
        text("Appearance").size(14),
        Space::new().height(8),
        row![
            theme_button(system_label, ThemeSetting::System, settings),
            theme_button("Light", ThemeSetting::Light, settings),
            theme_button("Dark", ThemeSetting::Dark, settings),
        ]
        .spacing(8),
        Space::new().height(28),
    ]
    .spacing(4)
    .into()
}

fn theme_button<'a>(
    label: &'a str,
    theme: ThemeSetting,
    settings: &'a AppSettings,
) -> iced::widget::Button<'a, Message> {
    button(text(label).size(13))
        .on_press(Message::ThemeChanged(theme))
        .style(if settings.theme == theme {
            styles::primary_button
        } else {
            styles::secondary_button
        })
        .padding([10, 16])
}

fn preferred_engine_section<'a>(
    settings: &'a AppSettings,
    state: &'a MainState,
) -> Element<'a, Message> {
    column![
        text("Preferred Engine").size(14),
        Space::new().height(8),
        engine_selector(settings, state),
        text(format!("Currently using: {}", state.backend_name))
            .size(11)
            .color(crate::theme::tokens::TEXT_MUTED),
        text("Each environment uses whichever engine is available")
            .size(11)
            .color(crate::theme::tokens::TEXT_MUTED),
        Space::new().height(28),
    ]
    .spacing(4)
    .into()
}

fn tray_section(settings: &AppSettings) -> Element<'_, Message> {
    column![
        text("System Tray").size(14),
        Space::new().height(8),
        row![
            tray_behavior_button("When Open", TrayBehavior::WhenWindowOpen, settings),
            tray_behavior_button("Always", TrayBehavior::AlwaysRunning, settings),
            tray_behavior_button("Disabled", TrayBehavior::Disabled, settings),
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
        launch_at_login_row(settings),
        text("\"Always\" keeps the app running in the tray when closed")
            .size(11)
            .color(crate::theme::tokens::TEXT_MUTED),
        Space::new().height(28),
    ]
    .spacing(4)
    .into()
}

fn update_behavior_section(settings: &AppSettings) -> Element<'_, Message> {
    column![
        text("App Updates").size(14),
        Space::new().height(8),
        row![
            update_behavior_button("Off", AppUpdateBehavior::DoNotCheck, settings),
            update_behavior_button(
                "Check Periodically",
                AppUpdateBehavior::CheckPeriodically,
                settings
            ),
            update_behavior_button(
                "Auto Update",
                AppUpdateBehavior::AutomaticallyUpdate,
                settings
            ),
        ]
        .spacing(8),
        text("Off: never check. Check Periodically: notify about updates. Auto Update: download and apply in background.")
            .size(11)
            .color(crate::theme::tokens::TEXT_MUTED),
        Space::new().height(28),
    ]
    .spacing(4)
    .into()
}

fn tray_behavior_button<'a>(
    label: &'a str,
    behavior: TrayBehavior,
    settings: &'a AppSettings,
) -> iced::widget::Button<'a, Message> {
    button(text(label).size(13))
        .on_press(Message::TrayBehaviorChanged(behavior))
        .style(if settings.tray_behavior == behavior {
            styles::primary_button
        } else {
            styles::secondary_button
        })
        .padding([10, 16])
}

fn update_behavior_button<'a>(
    label: &'a str,
    behavior: AppUpdateBehavior,
    settings: &'a AppSettings,
) -> iced::widget::Button<'a, Message> {
    button(text(label).size(13))
        .on_press(Message::AppUpdateBehaviorChanged(behavior))
        .style(if settings.app_update_behavior == behavior {
            styles::primary_button
        } else {
            styles::secondary_button
        })
        .padding([10, 16])
}

fn shell_options_section(
    capabilities: versi_backend::ManagerCapabilities,
    shell_opts: crate::settings::ShellOptions,
) -> Element<'static, Message> {
    let mut section = column![text("Shell Options").size(14), Space::new().height(8),].spacing(4);

    if capabilities.supports_auto_switch {
        section = section.push(shell_option_toggle(
            shell_opts.use_on_cd,
            "Auto-switch on cd",
            Message::ShellOptionUseOnCdToggled,
        ));
    }
    if capabilities.supports_resolve_engines {
        section = section.push(shell_option_toggle(
            shell_opts.resolve_engines,
            "Resolve engines from package.json",
            Message::ShellOptionResolveEnginesToggled,
        ));
    }
    if capabilities.supports_corepack {
        section = section.push(shell_option_toggle(
            shell_opts.corepack_enabled,
            "Enable corepack",
            Message::ShellOptionCorepackEnabledToggled,
        ));
    }

    if !capabilities.supports_auto_switch
        && !capabilities.supports_resolve_engines
        && !capabilities.supports_corepack
    {
        section = section.push(
            text("No shell options available for this engine")
                .size(12)
                .color(crate::theme::tokens::TEXT_MUTED),
        );
    } else {
        section = section.push(
            text("Options for new shell configurations")
                .size(11)
                .color(crate::theme::tokens::TEXT_MUTED),
        );
    }

    section.push(Space::new().height(28)).into()
}

fn shell_option_toggle<F>(value: bool, label: &str, on_toggle: F) -> iced::widget::Row<'_, Message>
where
    F: Fn(bool) -> Message + 'static,
{
    row![
        toggler(value).on_toggle(on_toggle).size(18),
        text(label).size(12),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
}

fn shell_setup_section(settings_state: &SettingsModalState) -> Element<'_, Message> {
    let mut section = column![text("Shell Setup").size(14), Space::new().height(8),].spacing(4);

    if settings_state.checking_shells {
        section = section.push(text("Checking shell configuration...").size(12));
    } else if settings_state.shell_statuses.is_empty() {
        section = section.push(text("No shells detected").size(12));
    } else {
        for shell in &settings_state.shell_statuses {
            section = section.push(shell_status_row(shell));
        }
    }

    section.push(Space::new().height(28)).into()
}

fn shell_status_row(shell: &crate::state::ShellSetupStatus) -> iced::widget::Row<'_, Message> {
    let status_text = match &shell.status {
        ShellVerificationStatus::Configured => "Configured",
        ShellVerificationStatus::NotConfigured => "Not configured",
        ShellVerificationStatus::NoConfigFile => "No config file",
        ShellVerificationStatus::FunctionalButNotInConfig => "Working (not in config)",
        ShellVerificationStatus::Error => "Error",
    };

    if shell.configuring {
        return row![
            text(&shell.shell_name)
                .size(13)
                .width(Length::Fixed(crate::theme::tokens::COL_SHELL_NAME)),
            text("Configuring...").size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
    }

    if matches!(
        shell.status,
        ShellVerificationStatus::Configured | ShellVerificationStatus::FunctionalButNotInConfig
    ) {
        let mut row = row![
            text(&shell.shell_name)
                .size(13)
                .width(Length::Fixed(crate::theme::tokens::COL_SHELL_NAME)),
            text(status_text)
                .size(12)
                .color(crate::theme::tokens::SUCCESS),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if matches!(shell.status, ShellVerificationStatus::Configured) {
            let check_icon: Element<'_, Message> = icon::check(12.0)
                .style(|_theme: &iced::Theme, _status| iced::widget::svg::Style {
                    color: Some(crate::theme::tokens::SUCCESS),
                })
                .into();
            row = row.push(check_icon);
        }
        return row;
    }

    if matches!(shell.status, ShellVerificationStatus::NoConfigFile) {
        return row![
            text(&shell.shell_name)
                .size(13)
                .width(Length::Fixed(crate::theme::tokens::COL_SHELL_NAME)),
            text(status_text)
                .size(12)
                .color(crate::theme::tokens::TEXT_MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
    }

    row![
        text(&shell.shell_name)
            .size(13)
            .width(Length::Fixed(crate::theme::tokens::COL_SHELL_NAME)),
        text(status_text)
            .size(12)
            .color(crate::theme::tokens::EOL_ORANGE),
        Space::new().width(Length::Fill),
        button(text("Configure").size(11))
            .on_press(Message::ConfigureShell(shell.shell_type))
            .style(styles::secondary_button)
            .padding([4, 10]),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
}

fn settings_data_section() -> Element<'static, Message> {
    column![
        text("Settings Data").size(14),
        Space::new().height(8),
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
        text("Export or import preferences, or edit the config file directly")
            .size(11)
            .color(crate::theme::tokens::TEXT_MUTED),
        Space::new().height(28),
    ]
    .spacing(4)
    .into()
}

fn advanced_section<'a>(
    settings_state: &'a SettingsModalState,
    settings: &'a AppSettings,
) -> Element<'a, Message> {
    let log_path = &settings_state.log_file_path;
    let log_size_text = match settings_state.log_file_size {
        Some(0) => "empty".to_string(),
        Some(size) if size < 1024 => format!("{size} B"),
        Some(size) if size < 1024 * 1024 => format_tenths(size, 1024, "KB"),
        Some(size) => format_tenths(size, 1024 * 1024, "MB"),
        None => "not found".to_string(),
    };

    column![
        text("Advanced").size(14),
        Space::new().height(8),
        row![
            toggler(settings.debug_logging)
                .on_toggle(Message::DebugLoggingToggled)
                .size(18),
            text("Debug logging").size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            text("Log file: ")
                .size(11)
                .color(crate::theme::tokens::TEXT_MUTED),
            button(text(log_path.clone()).size(11))
                .on_press(Message::CopyToClipboard(log_path.clone()))
                .style(styles::link_button)
                .padding(0),
            text(format!(" ({log_size_text})"))
                .size(11)
                .color(crate::theme::tokens::TEXT_MUTED),
        ]
        .align_y(Alignment::Center),
        Space::new().height(8),
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
    ]
    .spacing(4)
    .into()
}

fn engine_button<'a>(
    kind: BackendKind,
    is_selected: bool,
    is_detected: bool,
) -> Element<'a, Message> {
    let btn = button(text(kind.as_str()).size(13))
        .style(if is_selected {
            styles::primary_button
        } else {
            styles::secondary_button
        })
        .padding([10, 16]);

    if is_detected {
        btn.on_press(Message::PreferredBackendChanged(kind)).into()
    } else {
        tooltip(
            btn,
            container(text(format!("{kind} is not installed")).size(12))
                .padding([4, 8])
                .style(styles::tooltip_container),
            tooltip::Position::Bottom,
        )
        .gap(4.0)
        .into()
    }
}

fn launch_at_login_row(settings: &AppSettings) -> Element<'_, Message> {
    let is_always = settings.tray_behavior == TrayBehavior::AlwaysRunning;
    let toggle = if is_always {
        toggler(settings.launch_at_login)
            .on_toggle(Message::LaunchAtLoginToggled)
            .size(18)
    } else {
        toggler(false).size(18)
    };

    let label_color = if is_always {
        None
    } else {
        Some(crate::theme::tokens::TEXT_MUTED)
    };

    let mut label = text("Launch at login").size(12);
    if let Some(color) = label_color {
        label = label.color(color);
    }

    row![toggle, label]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
}

fn engine_selector<'a>(settings: &'a AppSettings, state: &'a MainState) -> Element<'a, Message> {
    let preferred = settings.preferred_backend.unwrap_or(BackendKind::DEFAULT);
    let fnm_detected = state.detected_backends.contains(&BackendKind::Fnm);
    let nvm_detected = state.detected_backends.contains(&BackendKind::Nvm);
    let volta_detected = state.detected_backends.contains(&BackendKind::Volta);
    let asdf_detected = state.detected_backends.contains(&BackendKind::Asdf);

    row![
        engine_button(
            BackendKind::Fnm,
            preferred == BackendKind::Fnm,
            fnm_detected
        ),
        engine_button(
            BackendKind::Nvm,
            preferred == BackendKind::Nvm,
            nvm_detected
        ),
        engine_button(
            BackendKind::Asdf,
            preferred == BackendKind::Asdf,
            asdf_detected
        ),
        engine_button(
            BackendKind::Volta,
            preferred == BackendKind::Volta,
            volta_detected
        ),
    ]
    .spacing(8)
    .into()
}
