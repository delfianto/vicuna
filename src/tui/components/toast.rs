use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Clone, Debug)]
pub struct Toast {
    pub message: String,
    pub duration: usize,
    pub color: Color,
}

pub fn draw(f: &mut Frame, toast: &Toast, area: Rect) {
    let width = 40;
    let height = 3;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = 1;

    let toast_area = Rect::new(x, y, width, height);

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(toast.color));

    let p = Paragraph::new(toast.message.clone()).block(block);

    f.render_widget(Clear, toast_area);
    f.render_widget(p, toast_area);
}