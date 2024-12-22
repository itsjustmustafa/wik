use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

pub struct TextBox {
    text: String,
    cursor_pos: usize,
    text_style: Style,
    cursor_style: Style,
}

impl TextBox {
    pub fn new(text: String, cursor_pos: usize) -> Self {
        Self {
            text,
            cursor_pos,
            text_style: Style::default().fg(Color::Black).bg(Color::White),
            cursor_style: Style::default().fg(Color::White).bg(Color::Black),
        }
    }

    pub fn cursor_style(mut self, style: Style) -> Self {
        self.cursor_style = style;
        self
    }

    pub fn text_style(mut self, style: Style) -> Self {
        self.text_style = style;
        self
    }
}

impl Widget for TextBox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // for x in (area.x) .. (area.width + area.x){

        // }
        let inner_area = {
            let block = Block::default().borders(Borders::ALL);
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        };

        let text_to_render = format!("{} ", self.text.clone().as_str());
        let text_len = text_to_render.len();
        if self.cursor_pos < inner_area.width as usize {
            for x in 0..text_len {
                if x >= inner_area.width as usize {
                    break;
                }
                let char = &text_to_render[x..x + 1];
                buf.set_string(
                    inner_area.x + x as u16,
                    inner_area.y,
                    char.to_string(),
                    if x == self.cursor_pos {
                        self.cursor_style
                    } else {
                        self.text_style
                    },
                );
            }
        } else {
            for x in 0..(inner_area.width) {
                let char_index =
                    (self.cursor_pos + (x + 1) as usize).saturating_sub(inner_area.width as usize);

                if char_index > text_len {
                    break;
                }
                buf.set_string(
                    inner_area.x + x,
                    inner_area.y,
                    text_to_render
                        .chars()
                        .nth(char_index)
                        .unwrap_or(' ')
                        .to_string(),
                    if char_index as usize == self.cursor_pos {
                        self.cursor_style
                    } else {
                        self.text_style
                        // Style::default().bg(Color::LightMagenta)
                    },
                );
            }
        }
    }
}
