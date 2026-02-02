pub mod styles;

use iced::theme::Palette;
use iced::{Theme, color};

pub mod tahoe {
    pub const RADIUS_SM: f32 = 8.0;
    pub const RADIUS_MD: f32 = 12.0;
    pub const RADIUS_LG: f32 = 16.0;

    pub fn card_bg(is_dark: bool) -> iced::Color {
        if is_dark {
            iced::Color::from_rgba8(44, 44, 46, 0.72)
        } else {
            iced::Color::from_rgba8(255, 255, 255, 0.72)
        }
    }
}

pub fn light_theme() -> Theme {
    Theme::custom(
        "Versi Light".to_string(),
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
        "Versi Dark".to_string(),
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
