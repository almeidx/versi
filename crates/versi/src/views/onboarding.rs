use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::{OnboardingState, OnboardingStep};
use crate::theme::styles;

pub fn view<'a>(state: &'a OnboardingState, backend_name: &'a str) -> Element<'a, Message> {
    let content = match state.step {
        OnboardingStep::Welcome => welcome_step(backend_name),
        OnboardingStep::SelectBackend => select_backend_step(state),
        OnboardingStep::InstallBackend => install_backend_step(state, backend_name),
        OnboardingStep::ConfigureShell => configure_shell_step(state, backend_name),
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
    let has_select = state.available_backends.len() > 1;

    let mut steps: Vec<(&str, OnboardingStep)> = vec![("Welcome", OnboardingStep::Welcome)];

    if has_select {
        steps.push(("Engine", OnboardingStep::SelectBackend));
    }

    steps.push(("Install", OnboardingStep::InstallBackend));
    steps.push(("Configure Shell", OnboardingStep::ConfigureShell));

    let indicators: Vec<Element<Message>> = steps
        .iter()
        .map(|(name, step)| {
            let is_current = &state.step == step;
            let is_past =
                full_step_index(&state.step, has_select) > full_step_index(step, has_select);

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

fn full_step_index(step: &OnboardingStep, has_select: bool) -> usize {
    match step {
        OnboardingStep::Welcome => 0,
        OnboardingStep::SelectBackend => 1,
        OnboardingStep::InstallBackend => {
            if has_select {
                2
            } else {
                1
            }
        }
        OnboardingStep::ConfigureShell => {
            if has_select {
                3
            } else {
                2
            }
        }
    }
}

fn welcome_step(backend_name: &str) -> Element<'_, Message> {
    column![
        text("Welcome to Versi").size(32),
        Space::new().height(16),
        text("Versi helps you manage Node.js versions with a simple graphical interface.").size(16),
        Space::new().height(8),
        text(format!(
            "We'll help you set up {} to get started.",
            backend_name
        ))
        .size(16),
    ]
    .spacing(8)
    .into()
}

fn select_backend_step<'a>(state: &'a OnboardingState) -> Element<'a, Message> {
    let mut content = column![
        text("Choose an Engine").size(28),
        Space::new().height(16),
        text("Select which Node.js version manager you'd like to use.").size(16),
        Space::new().height(24),
    ]
    .spacing(8);

    let selected = state.selected_backend.as_deref();

    for backend in &state.available_backends {
        let is_selected = selected == Some(backend.name);
        let name = backend.name.to_string();

        let btn_style = if is_selected {
            styles::primary_button
        } else {
            styles::secondary_button
        };

        let label = if backend.detected {
            format!("{} (detected)", backend.display_name)
        } else {
            backend.display_name.to_string()
        };

        content = content.push(
            button(text(label).size(14))
                .on_press(Message::OnboardingSelectBackend(name))
                .style(btn_style)
                .padding([12, 24])
                .width(Length::Fill),
        );
        content = content.push(Space::new().height(8));
    }

    content.into()
}

fn install_backend_step<'a>(
    state: &'a OnboardingState,
    backend_name: &str,
) -> Element<'a, Message> {
    let mut content = column![
        text(format!("Install {}", backend_name)).size(28),
        Space::new().height(16),
        text(format!(
            "{} needs to be installed on your system.",
            backend_name
        ))
        .size(16),
    ]
    .spacing(8);

    if state.backend_installing {
        content = content.push(
            row![text(format!("Installing {}...", backend_name)).size(16),]
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
                    .on_press(Message::OnboardingInstallBackend)
                    .style(styles::primary_button),
            ]
            .spacing(8),
        );
    } else {
        content = content.push(
            column![
                Space::new().height(24),
                button(text(format!("Install {}", backend_name)).size(16))
                    .on_press(Message::OnboardingInstallBackend)
                    .style(styles::primary_button)
                    .padding([12, 24]),
            ]
            .spacing(8),
        );
    }

    content.into()
}

fn configure_shell_step<'a>(
    state: &'a OnboardingState,
    backend_name: &str,
) -> Element<'a, Message> {
    let mut content = column![
        text("Configure Shell").size(28),
        Space::new().height(16),
        text(format!(
            "{} needs to be added to your shell configuration.",
            backend_name
        ))
        .size(16),
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
            } else if shell.config_path.is_none() {
                container(
                    text("No config file")
                        .size(14)
                        .color(iced::Color::from_rgb8(142, 142, 147)),
                )
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
        OnboardingStep::ConfigureShell => "Finish",
        _ => "Next",
    };

    let can_proceed = match state.step {
        OnboardingStep::SelectBackend => state.selected_backend.is_some(),
        OnboardingStep::InstallBackend => !state.backend_installing,
        OnboardingStep::ConfigureShell => state.detected_shells.iter().any(|s| s.configured),
        _ => true,
    };

    let next_message = if state.step == OnboardingStep::ConfigureShell {
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
