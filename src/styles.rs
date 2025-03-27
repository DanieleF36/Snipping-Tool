use crate::button::Appearance;
use iced::widget::button;
use iced::Theme;
use iced::{Color, Vector};
use iced_graphics::core::BorderRadius;

#[derive(Default)]
pub struct ButtonStyle {
    background: Color,
    text_color: Color,
}
impl ButtonStyle {
    pub fn new(background: Color) -> Self {
        Self {
            background,
            text_color: Color::BLACK,
        }
    }

    pub fn new_with_text_color(background: Color, text_color: Color) -> Self {
        Self {
            background,
            text_color,
        }
    }
}

impl button::StyleSheet for ButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        match style {
            _ => Appearance {
                shadow_offset: Vector::default(),
                background: Some(iced::Background::Color(self.background)),
                border_radius: BorderRadius::from(10f32),
                border_width: 1.0,
                border_color: Color::TRANSPARENT,
                text_color: self.text_color,
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        match style {
            _ => Appearance {
                shadow_offset: Vector::default(),
                background: Some(iced::Background::Color(Color::from_rgb8(168, 168, 168))),
                border_radius: BorderRadius::from(10f32),
                border_width: 1.0,
                border_color: Color::BLACK,
                text_color: self.text_color,
            },
        }
    }

    fn pressed(&self, style: &Self::Style) -> Appearance {
        match style {
            _ => Appearance {
                shadow_offset: Vector::default(),
                background: Some(iced::Background::Color(Color::from_rgb8(138, 138, 138))),
                border_radius: BorderRadius::from(10f32),
                border_width: 1.0,
                border_color: Color::BLACK,
                text_color: self.text_color,
            },
        }
    }

    fn disabled(&self, style: &Self::Style) -> Appearance {
        match style {
            _ => Appearance {
                shadow_offset: Vector::default(),
                background: Some(iced::Background::Color(Color::from_rgb8(90, 90, 90))),
                border_radius: BorderRadius::from(10f32),
                border_width: 1.0,
                border_color: Color::from_rgb8(255, 255, 255),
                text_color: self.text_color,
            },
        }
    }
}