use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::{OnboardingState, OnboardingStep};
use crate::theme::styles;

pub fn view<'a>(state: &'a OnboardingState) -> Element<'a, Message> {
    let content = match state.step {
        OnboardingStep::Welcome => welcome_step(),
        OnboardingStep::InstallFnm => install_fnm_step(state),
        OnboardingStep::ConfigureShell => configure_shell_step(state),
        OnboardingStep::InstallNode => install_node_step(state),
        OnboardingStep::Complete => complete_step(),
    };

    let progress = step_indicator(state);

    let nav_buttons = navigation_buttons(state);

    container(
        column![
            progress,
            content,
            Space::new().height(Length::Fill),
            nav_buttons,
        ]
        .spacing(32)
        .padding(48)
        .max_width(600),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn step_indicator<'a>(state: &'a OnboardingState) -> Element<'a, Message> {
    let steps = [
        ("Welcome", OnboardingStep::Welcome),
        ("Install fnm", OnboardingStep::InstallFnm),
        ("Configure Shell", OnboardingStep::ConfigureShell),
        ("Install Node", OnboardingStep::InstallNode),
        ("Complete", OnboardingStep::Complete),
    ];

    let indicators: Vec<Element<Message>> = steps
        .iter()
        .map(|(name, step)| {
            let is_current = &state.step == step;
            let is_past = step_index(&state.step) > step_index(step);

            let dot_color = if is_current || is_past {
                iced::Color::from_rgb(0.0, 0.5, 0.0)
            } else {
                iced::Color::from_rgb(0.7, 0.7, 0.7)
            };

            column![
                container(Space::new().width(12).height(12)).style(move |_theme| {
                    container::Style {
                        background: Some(iced::Background::Color(dot_color)),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
                text(*name).size(11),
            ]
            .spacing(4)
            .align_x(Alignment::Center)
            .into()
        })
        .collect();

    row(indicators)
        .spacing(24)
        .align_y(Alignment::Center)
        .into()
}

fn step_index(step: &OnboardingStep) -> usize {
    match step {
        OnboardingStep::Welcome => 0,
        OnboardingStep::InstallFnm => 1,
        OnboardingStep::ConfigureShell => 2,
        OnboardingStep::InstallNode => 3,
        OnboardingStep::Complete => 4,
    }
}

fn welcome_step() -> Element<'static, Message> {
    column![
        text("Welcome to fnm-ui").size(32),
        Space::new().height(16),
        text("fnm-ui helps you manage Node.js versions with a simple graphical interface.")
            .size(16),
        Space::new().height(8),
        text("We'll help you set up fnm (Fast Node Manager) to get started.").size(16),
        Space::new().height(24),
        text("fnm is a fast and simple Node.js version manager, built in Rust.").size(14),
    ]
    .spacing(8)
    .into()
}

fn install_fnm_step<'a>(state: &'a OnboardingState) -> Element<'a, Message> {
    let mut content = column![
        text("Install fnm").size(28),
        Space::new().height(16),
        text("fnm (Fast Node Manager) needs to be installed on your system.").size(16),
    ]
    .spacing(8);

    if state.fnm_installing {
        content = content.push(
            row![text("Installing fnm...").size(16),]
                .spacing(8)
                .align_y(Alignment::Center),
        );
    } else if let Some(error) = &state.install_error {
        content = content.push(
            column![
                text("Installation failed:").size(16),
                text(error).size(14),
                Space::new().height(16),
                button(text("Retry"))
                    .on_press(Message::OnboardingInstallFnm)
                    .style(styles::primary_button),
            ]
            .spacing(8),
        );
    } else {
        content = content.push(
            column![
                Space::new().height(24),
                button(text("Install fnm").size(16))
                    .on_press(Message::OnboardingInstallFnm)
                    .style(styles::primary_button)
                    .padding([12, 24]),
            ]
            .spacing(8),
        );
    }

    content.into()
}

fn configure_shell_step<'a>(state: &'a OnboardingState) -> Element<'a, Message> {
    let mut content = column![
        text("Configure Shell").size(28),
        Space::new().height(16),
        text("fnm needs to be added to your shell configuration.").size(16),
        Space::new().height(24),
    ]
    .spacing(8);

    for shell in &state.detected_shells {
        let shell_row = row![
            text(&shell.shell_name).size(16).width(Length::Fixed(120.0)),
            if shell.configured {
                container(text("Configured").size(14))
                    .padding([4, 8])
                    .style(crate::theme::styles::badge_lts)
            } else if shell.configuring {
                container(text("Configuring...").size(14))
            } else if let Some(error) = &shell.error {
                container(text(format!("Error: {}", error)).size(14))
            } else {
                container(
                    button(text("Configure").size(14))
                        .on_press(Message::OnboardingConfigureShell(shell.shell_type.clone()))
                        .style(styles::secondary_button)
                        .padding([6, 12]),
                )
            },
        ]
        .spacing(16)
        .align_y(Alignment::Center);

        content = content.push(shell_row);
        content = content.push(Space::new().height(8));
    }

    content.into()
}

fn install_node_step<'a>(_state: &'a OnboardingState) -> Element<'a, Message> {
    column![
        text("Install Node.js").size(28),
        Space::new().height(16),
        text("You can now install your first Node.js version.").size(16),
        Space::new().height(8),
        text("You can skip this step and install Node.js later from the main interface.").size(14),
    ]
    .spacing(8)
    .into()
}

fn complete_step() -> Element<'static, Message> {
    column![
        text("Setup Complete!").size(32),
        Space::new().height(16),
        text("fnm is now configured and ready to use.").size(16),
        Space::new().height(8),
        text("Click 'Finish' to start using fnm-ui.").size(16),
    ]
    .spacing(8)
    .into()
}

fn navigation_buttons<'a>(state: &'a OnboardingState) -> Element<'a, Message> {
    let back_button = if state.step != OnboardingStep::Welcome {
        button(text("Back"))
            .on_press(Message::OnboardingBack)
            .style(styles::secondary_button)
            .padding([10, 20])
    } else {
        button(text("Back"))
            .style(styles::secondary_button)
            .padding([10, 20])
    };

    let next_label = match state.step {
        OnboardingStep::Complete => "Finish",
        OnboardingStep::InstallNode => "Skip & Finish",
        _ => "Next",
    };

    let can_proceed = match state.step {
        OnboardingStep::InstallFnm => !state.fnm_installing,
        OnboardingStep::ConfigureShell => state.detected_shells.iter().any(|s| s.configured),
        _ => true,
    };

    let next_message = if state.step == OnboardingStep::Complete {
        Message::OnboardingComplete
    } else {
        Message::OnboardingNext
    };

    let next_button = if can_proceed {
        button(text(next_label))
            .on_press(next_message)
            .style(styles::primary_button)
            .padding([10, 20])
    } else {
        button(text(next_label))
            .style(styles::primary_button)
            .padding([10, 20])
    };

    row![back_button, Space::new().width(Length::Fill), next_button,]
        .spacing(16)
        .into()
}
