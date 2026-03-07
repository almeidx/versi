use iced::widget::button;
use iced::{Background, Border, Color, Shadow, Theme};

use super::{darken, lighten};

#[derive(Clone, Copy)]
struct TintedStyle {
    text: Color,
    bg: Color,
    bg_hovered: Color,
    bg_pressed: Color,
    radius: f32,
}

fn tinted_button(tint: TintedStyle, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(tint.bg)),
        text_color: tint.text,
        border: Border {
            radius: tint.radius.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(tint.bg_hovered)),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(tint.bg_pressed)),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.4,
                ..tint.text
            },
            ..base
        },
    }
}

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();

    let base = button::Style {
        background: Some(Background::Color(palette.primary)),
        text_color: Color::WHITE,
        border: Border {
            radius: crate::theme::tokens::RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow {
            color: Color {
                a: 0.15,
                ..palette.primary
            },
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(lighten(palette.primary, 0.05))),
            shadow: Shadow {
                color: Color {
                    a: 0.25,
                    ..palette.primary
                },
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(darken(palette.primary, 0.05))),
            shadow: Shadow {
                color: Color {
                    a: 0.1,
                    ..palette.primary
                },
                offset: iced::Vector::new(0.0, 1.0),
                blur_radius: 4.0,
            },
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color {
                a: 0.4,
                ..palette.primary
            })),
            text_color: Color {
                a: 0.6,
                ..Color::WHITE
            },
            shadow: Shadow::default(),
            ..base
        },
    }
}

pub fn danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let danger = crate::theme::tokens::DANGER;
    tinted_button(
        TintedStyle {
            text: danger,
            bg: Color::TRANSPARENT,
            bg_hovered: Color { a: 0.1, ..danger },
            bg_pressed: Color { a: 0.15, ..danger },
            radius: crate::theme::tokens::RADIUS_MD,
        },
        status,
    )
}

pub fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let is_dark = super::is_dark(theme);
    tinted_button(
        TintedStyle {
            text: palette.text,
            bg: if is_dark {
                Color::from_rgba8(255, 255, 255, 0.1)
            } else {
                Color::from_rgba8(0, 0, 0, 0.05)
            },
            bg_hovered: if is_dark {
                Color::from_rgba8(255, 255, 255, 0.15)
            } else {
                Color::from_rgba8(0, 0, 0, 0.08)
            },
            bg_pressed: if is_dark {
                Color::from_rgba8(255, 255, 255, 0.2)
            } else {
                Color::from_rgba8(0, 0, 0, 0.12)
            },
            radius: crate::theme::tokens::RADIUS_MD,
        },
        status,
    )
}

pub fn ghost_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();

    let base = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Color {
            a: 0.6,
            ..palette.text
        },
        border: Border {
            radius: crate::theme::tokens::RADIUS_XS.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            text_color: palette.text,
            background: Some(Background::Color(Color {
                a: 0.05,
                ..palette.text
            })),
            ..base
        },
        button::Status::Pressed => button::Style {
            text_color: palette.text,
            background: Some(Background::Color(Color {
                a: 0.1,
                ..palette.text
            })),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.3,
                ..palette.text
            },
            ..base
        },
    }
}

pub fn ghost_button_active(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();

    let base = button::Style {
        background: Some(Background::Color(Color {
            a: 0.08,
            ..palette.text
        })),
        text_color: palette.text,
        border: Border {
            radius: crate::theme::tokens::RADIUS_XS.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color {
                a: 0.12,
                ..palette.text
            })),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..palette.text
            })),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.3,
                ..palette.text
            },
            ..base
        },
    }
}

pub fn link_button(_theme: &Theme, status: button::Status) -> button::Style {
    let link_color = crate::theme::tokens::TEXT_MUTED;

    let base = button::Style {
        background: None,
        text_color: link_color,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            text_color: Color::from_rgb8(100, 100, 105),
            ..base
        },
        button::Status::Pressed => button::Style {
            text_color: Color::from_rgb8(80, 80, 85),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.4,
                ..link_color
            },
            ..base
        },
    }
}

pub fn update_badge_button(_theme: &Theme, status: button::Status) -> button::Style {
    let color = Color::from_rgb8(0, 122, 255);
    tinted_button(
        TintedStyle {
            text: color,
            bg: Color { a: 0.15, ..color },
            bg_hovered: Color { a: 0.25, ..color },
            bg_pressed: Color { a: 0.35, ..color },
            radius: crate::theme::tokens::RADIUS_XS,
        },
        status,
    )
}

pub fn app_update_button(theme: &Theme, status: button::Status) -> button::Style {
    let color = theme.palette().success;
    tinted_button(
        TintedStyle {
            text: color,
            bg: Color { a: 0.15, ..color },
            bg_hovered: Color { a: 0.25, ..color },
            bg_pressed: Color { a: 0.35, ..color },
            radius: crate::theme::tokens::RADIUS_XS,
        },
        status,
    )
}

pub fn active_tab_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();

    let base = button::Style {
        background: Some(Background::Color(palette.primary)),
        text_color: Color::WHITE,
        border: Border {
            radius: crate::theme::tokens::RADIUS_XS.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active | button::Status::Disabled => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(lighten(palette.primary, 0.05))),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(darken(palette.primary, 0.05))),
            ..base
        },
    }
}

pub fn inactive_tab_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let is_dark = super::is_dark(theme);

    let text_secondary = Color {
        a: 0.6,
        ..palette.text
    };

    let hover_bg = if is_dark {
        Color::from_rgba8(255, 255, 255, 0.1)
    } else {
        Color::from_rgba8(0, 0, 0, 0.05)
    };

    let base = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: text_secondary,
        border: Border {
            radius: crate::theme::tokens::RADIUS_XS.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active | button::Status::Disabled => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(hover_bg)),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba8(255, 255, 255, 0.15)
            } else {
                Color::from_rgba8(0, 0, 0, 0.08)
            })),
            ..base
        },
    }
}

pub fn disabled_tab_button(theme: &Theme, _status: button::Status) -> button::Style {
    let palette = theme.palette();

    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Color {
            a: 0.35,
            ..palette.text
        },
        border: Border {
            radius: crate::theme::tokens::RADIUS_XS.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn context_menu_item(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let is_dark = super::is_dark(theme);
    tinted_button(
        TintedStyle {
            text: palette.text,
            bg: Color::TRANSPARENT,
            bg_hovered: if is_dark {
                Color::from_rgba8(255, 255, 255, 0.08)
            } else {
                Color::from_rgba8(0, 0, 0, 0.05)
            },
            bg_pressed: if is_dark {
                Color::from_rgba8(255, 255, 255, 0.12)
            } else {
                Color::from_rgba8(0, 0, 0, 0.08)
            },
            radius: crate::theme::tokens::RADIUS_SM,
        },
        status,
    )
}

pub fn context_menu_item_danger(_theme: &Theme, status: button::Status) -> button::Style {
    let danger = crate::theme::tokens::DANGER;
    tinted_button(
        TintedStyle {
            text: danger,
            bg: Color::TRANSPARENT,
            bg_hovered: Color { a: 0.1, ..danger },
            bg_pressed: Color { a: 0.15, ..danger },
            radius: crate::theme::tokens::RADIUS_SM,
        },
        status,
    )
}

pub fn row_action_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();

    let base = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Color {
            a: 0.7,
            ..palette.text
        },
        border: Border {
            radius: crate::theme::tokens::RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color {
                a: 0.08,
                ..palette.text
            })),
            text_color: palette.text,
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color {
                a: 0.12,
                ..palette.text
            })),
            text_color: palette.text,
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.3,
                ..palette.text
            },
            ..base
        },
    }
}

pub fn banner_button_info(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();

    let base = button::Style {
        background: Some(Background::Color(Color {
            a: 0.1,
            ..palette.primary
        })),
        text_color: palette.primary,
        border: Border {
            radius: crate::theme::tokens::RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active | button::Status::Disabled => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color {
                a: 0.18,
                ..palette.primary
            })),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color {
                a: 0.25,
                ..palette.primary
            })),
            ..base
        },
    }
}

pub fn banner_button_warning(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let warning = palette.warning;

    let base = button::Style {
        background: Some(Background::Color(Color { a: 0.1, ..warning })),
        text_color: warning,
        border: Border {
            radius: crate::theme::tokens::RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active | button::Status::Disabled => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color { a: 0.18, ..warning })),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color { a: 0.25, ..warning })),
            ..base
        },
    }
}

pub fn row_action_button_hidden(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Color::TRANSPARENT,
        border: Border {
            radius: crate::theme::tokens::RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn row_action_button_danger(_theme: &Theme, status: button::Status) -> button::Style {
    let danger = crate::theme::tokens::DANGER;
    tinted_button(
        TintedStyle {
            text: danger,
            bg: Color::TRANSPARENT,
            bg_hovered: Color { a: 0.1, ..danger },
            bg_pressed: Color { a: 0.15, ..danger },
            radius: crate::theme::tokens::RADIUS_SM,
        },
        status,
    )
}

pub fn filter_chip(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let is_dark = super::is_dark(theme);

    let border_color = if is_dark {
        Color::from_rgba8(255, 255, 255, 0.12)
    } else {
        Color::from_rgba8(0, 0, 0, 0.1)
    };

    let base = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Color {
            a: 0.6,
            ..palette.text
        },
        border: Border {
            radius: crate::theme::tokens::RADIUS_SM.into(),
            width: 1.0,
            color: border_color,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            text_color: palette.text,
            background: Some(Background::Color(Color {
                a: 0.05,
                ..palette.text
            })),
            ..base
        },
        button::Status::Pressed => button::Style {
            text_color: palette.text,
            background: Some(Background::Color(Color {
                a: 0.1,
                ..palette.text
            })),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.3,
                ..palette.text
            },
            ..base
        },
    }
}

pub fn filter_chip_active(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();

    let base = button::Style {
        background: Some(Background::Color(Color {
            a: 0.12,
            ..palette.primary
        })),
        text_color: palette.primary,
        border: Border {
            radius: crate::theme::tokens::RADIUS_SM.into(),
            width: 1.0,
            color: Color {
                a: 0.25,
                ..palette.primary
            },
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color {
                a: 0.18,
                ..palette.primary
            })),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color {
                a: 0.25,
                ..palette.primary
            })),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.4,
                ..palette.primary
            },
            ..base
        },
    }
}
