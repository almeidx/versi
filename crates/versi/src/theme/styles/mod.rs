mod badges;
mod buttons;
mod containers;
mod scrollables;

pub use badges::*;
pub use buttons::*;
pub use containers::*;
pub use scrollables::*;

pub(crate) fn lighten(color: iced::Color, amount: f32) -> iced::Color {
    iced::Color {
        r: (color.r + amount).min(1.0),
        g: (color.g + amount).min(1.0),
        b: (color.b + amount).min(1.0),
        a: color.a,
    }
}

pub(crate) fn darken(color: iced::Color, amount: f32) -> iced::Color {
    iced::Color {
        r: (color.r - amount).max(0.0),
        g: (color.g - amount).max(0.0),
        b: (color.b - amount).max(0.0),
        a: color.a,
    }
}
