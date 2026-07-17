use crate::tui::styles;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

#[derive(Clone, Debug)]
pub struct Toast {
    pub message: String,
    pub duration: usize,
    pub color: ratatui::style::Color,
}

pub fn draw(f: &mut Frame, toast: &Toast, area: Rect) {
    let width = (toast.message.chars().count() as u16 + 6).clamp(24, area.width.saturating_sub(4));
    let height = 3;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + 1;

    let toast_area = Rect::new(x, y, width, height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(toast.color))
        .style(Style::default().bg(styles::BG_SURFACE));

    let p = Paragraph::new(Line::from(vec![
        Span::styled(
            " ● ",
            Style::default()
                .fg(toast.color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(toast.message.clone(), styles::text()),
    ]))
    .block(block);

    f.render_widget(Clear, toast_area);
    f.render_widget(p, toast_area);
}
