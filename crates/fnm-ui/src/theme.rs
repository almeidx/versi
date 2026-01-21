use iced::theme::Palette;
use iced::{color, Theme};

pub fn light_theme() -> Theme {
    Theme::custom(
        "fnm-ui Light".to_string(),
        Palette {
            background: color!(0xf5f5f7),
            text: color!(0x1d1d1f),
            primary: color!(0x007aff),
            success: color!(0x34c759),
            danger: color!(0xff3b30),
            warning: color!(0xff9500),
        },
    )
}

pub fn dark_theme() -> Theme {
    Theme::custom(
        "fnm-ui Dark".to_string(),
        Palette {
            background: color!(0x1c1c1e),
            text: color!(0xf5f5f7),
            primary: color!(0x0a84ff),
            success: color!(0x30d158),
            danger: color!(0xff453a),
            warning: color!(0xff9f0a),
        },
    )
}

pub fn get_system_theme() -> Theme {
    match dark_light::detect() {
        Ok(dark_light::Mode::Dark) => dark_theme(),
        Ok(dark_light::Mode::Light) | Ok(dark_light::Mode::Unspecified) => light_theme(),
        Err(_) => light_theme(),
    }
}

pub mod styles {
    use iced::widget::{button, container, text_input};
    use iced::{Background, Border, Color, Shadow, Theme};

    pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
        let palette = theme.palette();

        let base = button::Style {
            background: Some(Background::Color(palette.primary)),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
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
                radius: 8.0.into(),
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
                radius: 8.0.into(),
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

    pub fn card_container(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        let is_dark = palette.background.r < 0.5;

        let card_bg = if is_dark {
            Color::from_rgb8(44, 44, 46)
        } else {
            Color::WHITE
        };

        container::Style {
            background: Some(Background::Color(card_bg)),
            border: Border {
                radius: 12.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: Shadow {
                color: Color {
                    a: if is_dark { 0.3 } else { 0.08 },
                    ..Color::BLACK
                },
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 12.0,
            },
            text_color: None,
            snap: false,
        }
    }

    pub fn search_input(theme: &Theme, _status: text_input::Status) -> text_input::Style {
        let palette = theme.palette();
        let is_dark = palette.background.r < 0.5;

        let bg = if is_dark {
            Color::from_rgb8(44, 44, 46)
        } else {
            Color::from_rgb8(239, 239, 244)
        };

        let placeholder = Color {
            a: 0.4,
            ..palette.text
        };

        text_input::Style {
            background: Background::Color(bg),
            border: Border {
                radius: 10.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            icon: palette.text,
            placeholder,
            value: palette.text,
            selection: Color {
                a: 0.3,
                ..palette.primary
            },
        }
    }

    pub fn badge_default(theme: &Theme) -> container::Style {
        let palette = theme.palette();

        container::Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..palette.primary
            })),
            text_color: Some(palette.primary),
            border: Border {
                radius: 6.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
        }
    }

    pub fn badge_lts(theme: &Theme) -> container::Style {
        let palette = theme.palette();

        container::Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..palette.success
            })),
            text_color: Some(palette.success),
            border: Border {
                radius: 6.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn badge_update(_theme: &Theme) -> container::Style {
        let update_color = Color::from_rgb8(0, 122, 255);

        container::Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..update_color
            })),
            text_color: Some(update_color),
            border: Border {
                radius: 6.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
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

    fn lighten(color: Color, amount: f32) -> Color {
        Color {
            r: (color.r + amount).min(1.0),
            g: (color.g + amount).min(1.0),
            b: (color.b + amount).min(1.0),
            a: color.a,
        }
    }

    fn darken(color: Color, amount: f32) -> Color {
        Color {
            r: (color.r - amount).max(0.0),
            g: (color.g - amount).max(0.0),
            b: (color.b - amount).max(0.0),
            a: color.a,
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
}
