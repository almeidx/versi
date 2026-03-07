use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::backend_kind::BackendKind;
use crate::message::Message;
use crate::state::{OnboardingState, OnboardingStep};
use crate::theme::styles;

pub fn view(state: &OnboardingState, backend_name: BackendKind) -> Element<'_, Message> {
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
        .padding(iced::Padding::new(crate::theme::tokens::ONBOARDING_PADDING))
        .max_width(crate::theme::tokens::ONBOARDING_MAX_WIDTH),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn step_indicator(state: &OnboardingState) -> Element<'_, Message> {
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
                            radius: crate::theme::tokens::RADIUS_XS.into(),
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

fn welcome_step(backend_name: BackendKind) -> Element<'static, Message> {
    column![
        text("Welcome to Versi").size(32),
        Space::new().height(16),
        text("Versi helps you manage Node.js versions with a simple graphical interface.").size(16),
        Space::new().height(8),
        text(format!(
            "We'll help you set up {backend_name} to get started."
        ))
        .size(16),
    ]
    .spacing(8)
    .into()
}

fn select_backend_step(state: &OnboardingState) -> Element<'_, Message> {
    let mut content = column![
        text("Choose an Engine").size(28),
        Space::new().height(16),
        text("Select which Node.js version manager you'd like to use.").size(16),
        Space::new().height(24),
    ]
    .spacing(8);

    let selected = state.selected_backend;

    for backend in &state.available_backends {
        let is_selected = selected == Some(backend.kind);

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
                .on_press(Message::OnboardingSelectBackend(backend.kind))
                .style(btn_style)
                .padding([12, 24])
                .width(Length::Fill),
        );
        content = content.push(Space::new().height(8));
    }

    content.into()
}

fn install_backend_step(
    state: &OnboardingState,
    backend_name: BackendKind,
) -> Element<'_, Message> {
    let mut content = column![
        text(format!("Install {backend_name}")).size(28),
        Space::new().height(16),
        text(format!(
            "{backend_name} needs to be installed on your system."
        ))
        .size(16),
    ]
    .spacing(8);

    if state.backend_installing {
        content = content.push(
            row![text(format!("Installing {backend_name}...")).size(16),]
                .spacing(8)
                .align_y(Alignment::Center),
        );
    } else if state.confirming_unsafe_install {
        content = content.push(
            column![
                Space::new().height(16),
                text("No checksum verification will be performed.")
                    .size(14)
                    .color(crate::theme::tokens::DANGER),
                text("This may expose you to supply-chain risk.")
                    .size(14)
                    .color(crate::theme::tokens::DANGER),
                text("Continue only if you trust the source.")
                    .size(14)
                    .color(crate::theme::tokens::DANGER),
                Space::new().height(12),
                row![
                    button(text("Cancel").size(14))
                        .on_press(Message::OnboardingCancelInstallBackend)
                        .style(styles::secondary_button)
                        .padding([10, 18]),
                    button(text("Install Anyway").size(14))
                        .on_press(Message::OnboardingConfirmInstallBackend)
                        .style(styles::danger_button)
                        .padding([10, 18]),
                ]
                .spacing(10),
            ]
            .spacing(6),
        );
    } else if let Some(error) = &state.install_error {
        content = content.push(
            column![
                text("Installation failed:").size(16),
                text(error.to_string()).size(14),
                text("Installer downloads are not checksum-verified.")
                    .size(12)
                    .color(crate::theme::tokens::TEXT_MUTED),
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
                text("Installer downloads are not checksum-verified.")
                    .size(12)
                    .color(crate::theme::tokens::TEXT_MUTED),
                button(text(format!("Install {backend_name}")).size(16))
                    .on_press(Message::OnboardingInstallBackend)
                    .style(styles::primary_button)
                    .padding([12, 24]),
            ]
            .spacing(8),
        );
    }

    content.into()
}

fn configure_shell_step(
    state: &OnboardingState,
    backend_name: BackendKind,
) -> Element<'_, Message> {
    let mut content = column![
        text("Configure Shell").size(28),
        Space::new().height(16),
        text(format!(
            "{backend_name} needs to be added to your shell configuration."
        ))
        .size(16),
        Space::new().height(24),
    ]
    .spacing(8);

    for shell in &state.detected_shells {
        let shell_row = row![
            text(&shell.shell_name)
                .size(16)
                .width(Length::Fixed(crate::theme::tokens::COL_VERSION)),
            if shell.configured {
                container(text("Configured").size(14))
                    .padding([4, 8])
                    .style(crate::theme::styles::badge_lts)
            } else if shell.configuring {
                container(text("Configuring...").size(14))
            } else if let Some(error) = &shell.error {
                container(text(format!("Error: {error}")).size(14))
            } else if shell.config_path.is_none() {
                container(
                    text("No config file")
                        .size(14)
                        .color(crate::theme::tokens::TEXT_MUTED),
                )
            } else {
                container(
                    button(text("Configure").size(14))
                        .on_press(Message::OnboardingConfigureShell(shell.shell_type))
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

fn navigation_buttons(state: &OnboardingState) -> Element<'_, Message> {
    let back_button = if state.step == OnboardingStep::Welcome {
        button(text("Back"))
            .style(styles::secondary_button)
            .padding([10, 20])
    } else {
        button(text("Back"))
            .on_press(Message::OnboardingBack)
            .style(styles::secondary_button)
            .padding([10, 20])
    };

    let next_label = match state.step {
        OnboardingStep::ConfigureShell => "Finish",
        _ => "Next",
    };

    let can_proceed = match state.step {
        OnboardingStep::Welcome => true,
        OnboardingStep::SelectBackend => state.selected_backend.is_some(),
        OnboardingStep::InstallBackend => {
            !state.backend_installing
                && !state.confirming_unsafe_install
                && state.install_error.is_none()
        }
        OnboardingStep::ConfigureShell => state.detected_shells.iter().any(|s| s.configured),
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
