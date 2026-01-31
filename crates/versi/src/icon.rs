use iced::widget::svg;
use iced::{Color, Length, Theme};

fn themed_icon(bytes: &'static [u8], size: f32) -> svg::Svg<'static, Theme> {
    svg(svg::Handle::from_memory(bytes))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(|theme: &Theme, _status| {
            let palette = theme.palette();
            svg::Style {
                color: Some(Color {
                    a: 0.6,
                    ..palette.text
                }),
            }
        })
}

pub fn arrow_up_right(size: f32) -> svg::Svg<'static, Theme> {
    themed_icon(
        include_bytes!("../../../assets/icons/arrow-up-right.svg"),
        size,
    )
}

pub fn home(size: f32) -> svg::Svg<'static, Theme> {
    themed_icon(include_bytes!("../../../assets/icons/home.svg"), size)
}

pub fn refresh(size: f32) -> svg::Svg<'static, Theme> {
    themed_icon(include_bytes!("../../../assets/icons/refresh.svg"), size)
}

pub fn refresh_spinning(size: f32, radians: f32) -> svg::Svg<'static, Theme> {
    themed_icon(include_bytes!("../../../assets/icons/refresh.svg"), size).rotation(radians)
}

pub fn settings(size: f32) -> svg::Svg<'static, Theme> {
    themed_icon(include_bytes!("../../../assets/icons/settings.svg"), size)
}

pub fn info(size: f32) -> svg::Svg<'static, Theme> {
    themed_icon(include_bytes!("../../../assets/icons/info.svg"), size)
}

pub fn close(size: f32) -> svg::Svg<'static, Theme> {
    themed_icon(include_bytes!("../../../assets/icons/close.svg"), size)
}

pub fn check(size: f32) -> svg::Svg<'static, Theme> {
    themed_icon(include_bytes!("../../../assets/icons/check.svg"), size)
}

pub fn chevron_down(size: f32) -> svg::Svg<'static, Theme> {
    themed_icon(
        include_bytes!("../../../assets/icons/chevron-down.svg"),
        size,
    )
}

pub fn chevron_right(size: f32) -> svg::Svg<'static, Theme> {
    themed_icon(
        include_bytes!("../../../assets/icons/chevron-right.svg"),
        size,
    )
}
