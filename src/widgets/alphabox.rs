use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

use crate::utils::blended_color;

pub struct AlphaBox {
    color: Color,
    alpha: u8,
}

impl AlphaBox {
    pub fn new(color: Color, alpha: u8) -> Self {
        Self { color, alpha }
    }
}

impl Widget for AlphaBox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for x in (area.x)..(area.width + area.x) {
            for y in (area.y)..(area.height + area.y) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(blended_color(cell.bg, self.color, self.alpha));
                    cell.set_fg(blended_color(cell.fg, self.color, self.alpha));
                }
            }
        }
    }
}
