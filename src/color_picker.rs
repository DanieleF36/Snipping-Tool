use iced::widget::radio::Appearance;
use iced::widget::{radio, row};
use iced::{theme, Color, Element};

pub const COLORS: [Color; 8] = [
    Color::WHITE,
    Color::BLACK,
    Color {
        r: 0.81,
        g: 0.18,
        b: 0.18,
        a: 1.0,
    },
    Color {
        r: 1.0,
        g: 0.41,
        b: 0.0,
        a: 1.0,
    },
    Color {
        r: 0.99,
        g: 0.73,
        b: 0.0,
        a: 1.0,
    },
    Color {
        r: 0.0,
        g: 0.82,
        b: 0.52,
        a: 1.0,
    },
    Color {
        r: 0.02,
        g: 0.58,
        b: 0.89,
        a: 1.0,
    },
    Color {
        r: 0.61,
        g: 0.32,
        b: 0.88,
        a: 1.0,
    },
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ToolColor {
    White = 0,
    Black,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
}

impl Into<Color> for ToolColor {
    fn into(self) -> Color {
        COLORS[self as usize]
    }
}

#[derive(Default)]
pub struct ColorRadio {
    color: Color,
}

pub struct ColorPicker {}

impl ColorPicker {
    pub fn view<'a, Message: 'a>(
        &'a self,
        selected: Option<ToolColor>,
        on_click: impl Fn(ToolColor) -> Message,
    ) -> Element<'a, Message>
    where
        Message: std::clone::Clone,
    {
        row![
            iced::widget::radio("", ToolColor::White, selected, &on_click).style(
                theme::Radio::Custom(Box::new(ColorRadio::new(ToolColor::White.into())))
            ),
            iced::widget::radio("", ToolColor::Black, selected, &on_click).style(
                theme::Radio::Custom(Box::new(ColorRadio::new(ToolColor::Black.into())))
            ),
            iced::widget::radio("", ToolColor::Red, selected, &on_click).style(
                theme::Radio::Custom(Box::new(ColorRadio::new(ToolColor::Red.into())))
            ),
            iced::widget::radio("", ToolColor::Orange, selected, &on_click).style(
                theme::Radio::Custom(Box::new(ColorRadio::new(ToolColor::Orange.into())))
            ),
            iced::widget::radio("", ToolColor::Yellow, selected, &on_click).style(
                theme::Radio::Custom(Box::new(ColorRadio::new(ToolColor::Yellow.into())))
            ),
            iced::widget::radio("", ToolColor::Green, selected, &on_click).style(
                theme::Radio::Custom(Box::new(ColorRadio::new(ToolColor::Green.into())))
            ),
            iced::widget::radio("", ToolColor::Blue, selected, &on_click).style(
                theme::Radio::Custom(Box::new(ColorRadio::new(ToolColor::Blue.into())))
            ),
            iced::widget::radio("", ToolColor::Violet, selected, &on_click).style(
                theme::Radio::Custom(Box::new(ColorRadio::new(ToolColor::Violet.into())))
            ),
        ]
        .into()
    }
}

const SELECTED: Color = Color {
    r: 0.47,
    g: 0.75,
    b: 1.0,
    a: 1.0,
};
const DESELECTED: Color = Color {
    r: 0.43,
    g: 0.46,
    b: 0.51,
    a: 1.0,
};

impl ColorRadio {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

impl radio::StyleSheet for ColorRadio {
    type Style = theme::Theme;

    fn active(&self, style: &Self::Style, is_selected: bool) -> Appearance {
        match style {
            _ => Appearance {
                background: iced::Background::Color(self.color),
                border_width: 3.0,
                border_color: if is_selected { SELECTED } else { DESELECTED },
                dot_color: self.color,
                text_color: None,
            },
        }
    }

    fn hovered(&self, style: &Self::Style, is_selected: bool) -> Appearance {
        match style {
            _ => Appearance {
                background: iced::Background::Color(self.color),
                border_width: 3.0,
                border_color: if is_selected { SELECTED } else { DESELECTED },
                dot_color: self.color,
                text_color: None,
            },
        }
    }
}
