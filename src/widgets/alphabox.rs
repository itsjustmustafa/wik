use tui::{style::Color, widgets::Widget};

use crate::utils::blendedColor;

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
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        for x in (area.x)..(area.width + area.x) {
            for y in (area.y)..(area.height + area.y) {
                let cell = buf.get_mut(x, y);
                cell.set_bg(blendedColor(cell.bg, self.color, self.alpha));
                cell.set_fg(blendedColor(cell.fg, self.color, self.alpha));
                // cell.set_bg(Color::Black);
                // cell.set_fg(Color::White);
            }
        }
    }
}
