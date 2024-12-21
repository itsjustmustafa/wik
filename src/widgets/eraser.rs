use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::Widget,
};

pub struct Eraser {}

impl Widget for Eraser {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for x in (area.x)..(area.width + area.x) {
            for y in (area.y)..(area.height + area.y) {
                let cell = buf.get_mut(x, y);
                cell.set_char(' ');
                cell.set_style(Style::default().remove_modifier(Modifier::all()));
            }
        }
    }
}
