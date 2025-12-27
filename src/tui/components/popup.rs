use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, Paragraph},
    style::{Style, Color},
};
use tui_textarea::TextArea;

pub struct Popup<'a> {
    pub title: String,
    pub textarea: TextArea<'a>,
}

impl<'a> Popup<'a> {
    pub fn new(title: String) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_block(Block::default().borders(Borders::ALL).title("Input"));
        Self { title, textarea }
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let area = centered_rect(60, 20, area);
        
        f.render_widget(Clear, area); // Clear background
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
            .split(area);
            
        f.render_widget(&self.textarea, chunks[0]);
        
        let help = Paragraph::new("Enter: Submit | Esc: Cancel")
            .style(Style::default().fg(Color::White));
        f.render_widget(help, chunks[1]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
