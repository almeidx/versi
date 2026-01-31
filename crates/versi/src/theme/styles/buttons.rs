use iced::widget::button;
use iced::{Background, Border, Color, Shadow, Theme};

use super::{darken, lighten};

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();

    let base = button::Style {
        background: Some(Background::Color(palette.primary)),
        text_color: Color::WHITE,
        border: Border {
            radius: crate::theme::tahoe::RADIUS_MD.into(),
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
    let danger_muted = Color::from_rgb8(255, 69, 58);

    let base = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: danger_muted,
        border: Border {
            radius: crate::theme::tahoe::RADIUS_MD.into(),
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
                r: 1.0,
                g: 0.27,
                b: 0.23,
                a: 0.1,
            })),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color {
                r: 1.0,
                g: 0.27,
                b: 0.23,
                a: 0.15,
            })),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.4,
                ..danger_muted
            },
            ..base
        },
    }
}

pub fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let is_dark = palette.background.r < 0.5;

    let bg_color = if is_dark {
        Color::from_rgba8(255, 255, 255, 0.1)
    } else {
        Color::from_rgba8(0, 0, 0, 0.05)
    };

    let hover_bg = if is_dark {
        Color::from_rgba8(255, 255, 255, 0.15)
    } else {
        Color::from_rgba8(0, 0, 0, 0.08)
    };

    let base = button::Style {
        background: Some(Background::Color(bg_color)),
        text_color: palette.text,
        border: Border {
            radius: crate::theme::tahoe::RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(hover_bg)),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba8(255, 255, 255, 0.2)
            } else {
                Color::from_rgba8(0, 0, 0, 0.12)
            })),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.4,
                ..palette.text
            },
            ..base
        },
    }
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
            radius: 6.0.into(),
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
            radius: 6.0.into(),
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
    let link_color = Color::from_rgb8(142, 142, 147);

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
    let update_color = Color::from_rgb8(0, 122, 255);

    let base = button::Style {
        background: Some(Background::Color(Color {
            a: 0.15,
            ..update_color
        })),
        text_color: update_color,
        border: Border {
            radius: 6.0.into(),
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
                a: 0.25,
                ..update_color
            })),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color {
                a: 0.35,
                ..update_color
            })),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.4,
                ..update_color
            },
            ..base
        },
    }
}

pub fn app_update_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let update_color = palette.success;

    let base = button::Style {
        background: Some(Background::Color(Color {
            a: 0.15,
            ..update_color
        })),
        text_color: update_color,
        border: Border {
            radius: 6.0.into(),
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
                a: 0.25,
                ..update_color
            })),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color {
                a: 0.35,
                ..update_color
            })),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.4,
                ..update_color
            },
            ..base
        },
    }
}

pub fn active_tab_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();

    let base = button::Style {
        background: Some(Background::Color(palette.primary)),
        text_color: Color::WHITE,
        border: Border {
            radius: 6.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(lighten(palette.primary, 0.05))),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(darken(palette.primary, 0.05))),
            ..base
        },
        button::Status::Disabled => base,
    }
}

pub fn inactive_tab_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let is_dark = palette.background.r < 0.5;

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
            radius: 6.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
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
        button::Status::Disabled => base,
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
            radius: 6.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    }
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
            radius: crate::theme::tahoe::RADIUS_SM.into(),
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
            radius: crate::theme::tahoe::RADIUS_MD.into(),
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
        button::Status::Disabled => base,
    }
}

pub fn banner_button_warning(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let warning = palette.warning;

    let base = button::Style {
        background: Some(Background::Color(Color { a: 0.1, ..warning })),
        text_color: warning,
        border: Border {
            radius: crate::theme::tahoe::RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color { a: 0.18, ..warning })),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color { a: 0.25, ..warning })),
            ..base
        },
        button::Status::Disabled => base,
    }
}

pub fn row_action_button_hidden(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Color::TRANSPARENT,
        border: Border {
            radius: crate::theme::tahoe::RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn row_action_button_danger(_theme: &Theme, status: button::Status) -> button::Style {
    let danger = Color::from_rgb8(255, 69, 58);

    let base = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: danger,
        border: Border {
            radius: crate::theme::tahoe::RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color { a: 0.1, ..danger })),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color { a: 0.15, ..danger })),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color { a: 0.4, ..danger },
            ..base
        },
    }
}
