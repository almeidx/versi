use iced::widget::scrollable;
use iced::{Background, Border, Color, Theme};

pub const OVERLAY_SCROLLBAR_WIDTH: f32 = 6.0;
pub const OVERLAY_SCROLLBAR_MARGIN: f32 = 2.0;
pub const OVERLAY_SCROLLBAR_LANE_WIDTH: f32 =
    OVERLAY_SCROLLBAR_WIDTH + (OVERLAY_SCROLLBAR_MARGIN * 2.0);

#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
struct AxisState {
    hovered: bool,
    dragged: bool,
    disabled: bool,
}

pub fn overlay_scrollbar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new()
        .width(OVERLAY_SCROLLBAR_WIDTH)
        .scroller_width(OVERLAY_SCROLLBAR_WIDTH)
        .margin(OVERLAY_SCROLLBAR_MARGIN)
}

pub fn overlay_scrollable(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);
    let palette = theme.palette();

    style.vertical_rail = rail_for_axis(palette, axis_state(status, Axis::Vertical));
    style.horizontal_rail = rail_for_axis(palette, axis_state(status, Axis::Horizontal));

    style
}

fn axis_state(status: scrollable::Status, axis: Axis) -> AxisState {
    match status {
        scrollable::Status::Active {
            is_horizontal_scrollbar_disabled,
            is_vertical_scrollbar_disabled,
        } => AxisState {
            hovered: false,
            dragged: false,
            disabled: match axis {
                Axis::Horizontal => is_horizontal_scrollbar_disabled,
                Axis::Vertical => is_vertical_scrollbar_disabled,
            },
        },
        scrollable::Status::Hovered {
            is_horizontal_scrollbar_hovered,
            is_vertical_scrollbar_hovered,
            is_horizontal_scrollbar_disabled,
            is_vertical_scrollbar_disabled,
        } => AxisState {
            hovered: match axis {
                Axis::Horizontal => is_horizontal_scrollbar_hovered,
                Axis::Vertical => is_vertical_scrollbar_hovered,
            },
            dragged: false,
            disabled: match axis {
                Axis::Horizontal => is_horizontal_scrollbar_disabled,
                Axis::Vertical => is_vertical_scrollbar_disabled,
            },
        },
        scrollable::Status::Dragged {
            is_horizontal_scrollbar_dragged,
            is_vertical_scrollbar_dragged,
            is_horizontal_scrollbar_disabled,
            is_vertical_scrollbar_disabled,
        } => AxisState {
            hovered: false,
            dragged: match axis {
                Axis::Horizontal => is_horizontal_scrollbar_dragged,
                Axis::Vertical => is_vertical_scrollbar_dragged,
            },
            disabled: match axis {
                Axis::Horizontal => is_horizontal_scrollbar_disabled,
                Axis::Vertical => is_vertical_scrollbar_disabled,
            },
        },
    }
}

fn rail_for_axis(palette: iced::theme::Palette, axis: AxisState) -> scrollable::Rail {
    let thumb_color = if axis.disabled {
        Color::TRANSPARENT
    } else if axis.dragged {
        Color {
            a: 0.85,
            ..palette.primary
        }
    } else if axis.hovered {
        Color {
            a: 0.45,
            ..palette.text
        }
    } else {
        Color {
            a: 0.28,
            ..palette.text
        }
    };

    let rail_background = if axis.disabled {
        None
    } else if axis.dragged {
        Some(Background::Color(Color {
            a: 0.12,
            ..palette.text
        }))
    } else if axis.hovered {
        Some(Background::Color(Color {
            a: 0.08,
            ..palette.text
        }))
    } else {
        Some(Background::Color(Color::TRANSPARENT))
    };

    scrollable::Rail {
        background: rail_background,
        border: rounded_border(),
        scroller: scrollable::Scroller {
            background: Background::Color(thumb_color),
            border: rounded_border(),
        },
    }
}

fn rounded_border() -> Border {
    Border {
        radius: 999.0.into(),
        width: 0.0,
        color: Color::TRANSPARENT,
    }
}

#[cfg(test)]
mod tests {
    use super::overlay_scrollable;
    use iced::widget::scrollable;
    use iced::{Background, Color};

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    fn background_color(background: Background) -> Color {
        match background {
            Background::Color(color) => color,
            Background::Gradient(_) => panic!("expected Background::Color"),
        }
    }

    fn optional_background_color(background: Option<Background>) -> Option<Color> {
        background.map(background_color)
    }

    #[test]
    fn disabled_axis_hides_rail_and_scroller() {
        let theme = crate::theme::light_theme();
        let style = overlay_scrollable(
            &theme,
            scrollable::Status::Active {
                is_horizontal_scrollbar_disabled: false,
                is_vertical_scrollbar_disabled: true,
            },
        );

        assert!(style.vertical_rail.background.is_none());
        let vertical_thumb = background_color(style.vertical_rail.scroller.background);
        assert_close(vertical_thumb.a, 0.0);

        let horizontal_thumb = background_color(style.horizontal_rail.scroller.background);
        assert_close(horizontal_thumb.a, 0.28);
    }

    #[test]
    fn hovered_state_only_boosts_hovered_axis() {
        let theme = crate::theme::light_theme();
        let active_style = overlay_scrollable(
            &theme,
            scrollable::Status::Active {
                is_horizontal_scrollbar_disabled: false,
                is_vertical_scrollbar_disabled: false,
            },
        );
        let hovered_style = overlay_scrollable(
            &theme,
            scrollable::Status::Hovered {
                is_horizontal_scrollbar_hovered: false,
                is_vertical_scrollbar_hovered: true,
                is_horizontal_scrollbar_disabled: false,
                is_vertical_scrollbar_disabled: false,
            },
        );

        let vertical_thumb = background_color(hovered_style.vertical_rail.scroller.background);
        assert_close(vertical_thumb.a, 0.45);
        let vertical_rail = optional_background_color(hovered_style.vertical_rail.background)
            .expect("vertical rail should be visible while hovered");
        assert_close(vertical_rail.a, 0.08);

        let hovered_horizontal_thumb =
            background_color(hovered_style.horizontal_rail.scroller.background);
        let active_horizontal_thumb =
            background_color(active_style.horizontal_rail.scroller.background);
        assert_close(hovered_horizontal_thumb.a, active_horizontal_thumb.a);

        let hovered_horizontal_rail =
            optional_background_color(hovered_style.horizontal_rail.background)
                .expect("horizontal rail should be present");
        let active_horizontal_rail =
            optional_background_color(active_style.horizontal_rail.background)
                .expect("horizontal rail should be present");
        assert_close(hovered_horizontal_rail.a, active_horizontal_rail.a);
    }

    #[test]
    fn dragged_state_uses_primary_thumb_on_dragged_axis() {
        let theme = crate::theme::light_theme();
        let palette = theme.palette();
        let style = overlay_scrollable(
            &theme,
            scrollable::Status::Dragged {
                is_horizontal_scrollbar_dragged: false,
                is_vertical_scrollbar_dragged: true,
                is_horizontal_scrollbar_disabled: false,
                is_vertical_scrollbar_disabled: false,
            },
        );

        let dragged_vertical_thumb = background_color(style.vertical_rail.scroller.background);
        assert_close(dragged_vertical_thumb.a, 0.85);
        assert_close(dragged_vertical_thumb.r, palette.primary.r);
        assert_close(dragged_vertical_thumb.g, palette.primary.g);
        assert_close(dragged_vertical_thumb.b, palette.primary.b);
    }
}
