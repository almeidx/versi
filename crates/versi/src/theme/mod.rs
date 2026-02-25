pub mod styles;

use iced::theme::Palette;
use iced::{Theme, color};

pub mod tokens {
    pub const RADIUS_XS: f32 = 6.0;
    pub const RADIUS_SM: f32 = 8.0;
    pub const RADIUS_MD: f32 = 12.0;
    pub const RADIUS_LG: f32 = 16.0;

    pub const TEXT_MUTED: iced::Color =
        iced::Color::from_rgb(142.0 / 255.0, 142.0 / 255.0, 147.0 / 255.0);
    pub const DANGER: iced::Color = iced::Color::from_rgb(1.0, 69.0 / 255.0, 58.0 / 255.0);
    pub const EOL_ORANGE: iced::Color = iced::Color::from_rgb(1.0, 149.0 / 255.0, 0.0);

    pub const COL_VERSION: f32 = 120.0;
    pub const COL_SHELL_NAME: f32 = 100.0;
    pub const COL_DATE: f32 = 80.0;
    pub const COL_KBD_KEY: f32 = 80.0;
    pub const COL_META_LABEL: f32 = 64.0;

    pub const CONTEXT_MENU_WIDTH: f32 = 180.0;
    pub const TOAST_MAX_WIDTH: f32 = 400.0;
    pub const MODAL_MAX_WIDTH: f32 = 480.0;
    pub const ONBOARDING_MAX_WIDTH: f32 = 600.0;

    pub const INSET_RIGHT: f32 = 24.0;
    pub const SCROLL_CONTENT_RIGHT_INSET: f32 = 8.0;
    pub const MODAL_PADDING: f32 = 28.0;
    pub const ONBOARDING_PADDING: f32 = 48.0;
    pub const GROUP_INDENT: f32 = 24.0;

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
            background: color!(0x00f5_f5f7),
            text: color!(0x001d_1d1f),
            primary: color!(0x0000_7aff),
            success: color!(0x0034_c759),
            danger: color!(0x00ff_3b30),
            warning: color!(0x00ff_9500),
        },
    )
}

pub fn dark_theme() -> Theme {
    Theme::custom(
        "Versi Dark".to_string(),
        Palette {
            background: color!(0x001c_1c1e),
            text: color!(0x00f5_f5f7),
            primary: color!(0x000a_84ff),
            success: color!(0x0030_d158),
            danger: color!(0x00ff_453a),
            warning: color!(0x00ff_9f0a),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::tokens;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn tokens_radius_constants_are_stable() {
        assert_close(tokens::RADIUS_XS, 6.0);
        assert_close(tokens::RADIUS_SM, 8.0);
        assert_close(tokens::RADIUS_MD, 12.0);
        assert_close(tokens::RADIUS_LG, 16.0);
    }

    #[test]
    fn tokens_color_constants_are_stable() {
        assert_close(tokens::TEXT_MUTED.r, 142.0 / 255.0);
        assert_close(tokens::TEXT_MUTED.g, 142.0 / 255.0);
        assert_close(tokens::TEXT_MUTED.b, 147.0 / 255.0);

        assert_close(tokens::DANGER.r, 1.0);
        assert_close(tokens::DANGER.g, 69.0 / 255.0);
        assert_close(tokens::DANGER.b, 58.0 / 255.0);

        assert_close(tokens::EOL_ORANGE.r, 1.0);
        assert_close(tokens::EOL_ORANGE.g, 149.0 / 255.0);
        assert_close(tokens::EOL_ORANGE.b, 0.0);
    }

    #[test]
    fn tokens_card_background_uses_expected_light_color() {
        let color = tokens::card_bg(false);

        assert_close(color.r, 1.0);
        assert_close(color.g, 1.0);
        assert_close(color.b, 1.0);
        assert_close(color.a, 0.72);
    }

    #[test]
    fn tokens_card_background_uses_expected_dark_color() {
        let color = tokens::card_bg(true);

        assert_close(color.r, 44.0 / 255.0);
        assert_close(color.g, 44.0 / 255.0);
        assert_close(color.b, 46.0 / 255.0);
        assert_close(color.a, 0.72);
    }
}
