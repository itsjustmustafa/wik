use serde::{Deserialize, Serialize};
use tui::style::{Color, Modifier, Style};

use crate::utils::hex_to_rgb;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Theme {
    pub name: String,
    pub background: Color,
    pub text: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub highlight: Color,
    pub negative_text: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // 2a3138-ffffff-c19c00-13a10e-3b78ff-000000
            name: String::from("Default"),
            background: Color::Rgb(42, 49, 56),
            text: Color::White,
            secondary: Color::Yellow,
            tertiary: Color::Green,
            highlight: Color::LightBlue,
            negative_text: Color::LightYellow,
        }
    }
}

impl Theme {
    pub fn highlighted_snippet_style(&self) -> Style {
        Style::default().bg(self.highlight).fg(self.negative_text)
    }

    pub fn unhighlighted_snippet_style(&self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn cursor_style(&self) -> Style {
        Style::default().bg(self.secondary).fg(self.negative_text)
    }

    pub fn highlighted_title_style(&self) -> Style {
        Style::default()
            .bg(self.secondary)
            .fg(self.negative_text)
            .add_modifier(Modifier::UNDERLINED)
    }

    pub fn unhighlighted_title_style(&self) -> Style {
        Style::default()
            .fg(self.tertiary)
            .add_modifier(Modifier::UNDERLINED)
    }

    pub fn window_background(&self) -> Style {
        Style::default().bg(self.background)
    }

    pub fn selected_option(&self) -> Style {
        Style::default()
            .fg(self.secondary)
            .add_modifier(Modifier::UNDERLINED)
    }

    pub fn unselected_option(&self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn loading(&self) -> Style {
        Style::default()
            .fg(self.secondary)
            .add_modifier(Modifier::ITALIC)
    }

    pub fn block_border_unfocus(&self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn block_border_focus(&self) -> Style {
        Style::default().fg(self.secondary)
    }

    // pub fn title_

    pub fn from_hex_string_series(name: String, hex_string_series: String) -> Theme {
        let theme_colors: Vec<Color> = hex_string_series
            .split('-')
            .map(|hex_string| -> Option<Color> {
                if let Ok(color) = hex_to_rgb(hex_string) {
                    Some(color)
                } else {
                    None
                }
            })
            .filter(|maybe_color| maybe_color.is_some())
            .map(|some_color| some_color.expect("Color parsing error"))
            .collect();

        if theme_colors.len() >= 6 {
            return Theme {
                name: name,
                background: theme_colors[0],
                text: theme_colors[1],
                secondary: theme_colors[2],
                tertiary: theme_colors[3],
                highlight: theme_colors[4],
                negative_text: theme_colors[5],
            };
        }
        Theme::default()
    }
}
