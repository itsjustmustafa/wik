use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

pub struct ScrollBar {
    pub height: usize,
    pub position: usize,
    pub total_items: usize,
    pub handle_height_percent: u8,
    _bar_style: Style,
    _handle_style: Style,
}

impl ScrollBar {
    pub fn new(height: usize, position: usize, total_items: usize) -> Self {
        Self {
            height: height,
            position: position,
            total_items: total_items,
            handle_height_percent: 20,
            _bar_style: Style::default().fg(Color::DarkGray),
            _handle_style: Style::default().fg(Color::Gray),
        }
    }

    pub fn bar_style(mut self, bar_style: Style) -> Self {
        self._bar_style = bar_style;
        self
    }

    pub fn handle_style(mut self, handle_style: Style) -> Self {
        self._handle_style = handle_style;
        self
    }

    fn calculate_handle(&self) -> (usize, usize) {
        let handle_size =
            ((self.handle_height_percent as f64 / 100.0) * self.height as f64).round() as usize;
        let handle_position = ((self.position as f64 / self.total_items as f64)
            * (self.height - handle_size) as f64)
            .round() as usize;
        if self.position + 1 >= self.total_items && self.position > 0 {
            return (handle_size, self.height - handle_size);
        }
        (handle_size, handle_position)
    }
}

impl Widget for ScrollBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for y in 0..self.height.min(area.height as usize) {
            buf.set_string(area.x as u16, area.y + y as u16, "║", self._bar_style);
        }
        let (handle_size, handle_position) = self.calculate_handle();
        for y in handle_position..(handle_position + handle_size) {
            buf.set_string(area.x as u16, area.y + y as u16, "█", self._handle_style);
        }
    }
}
