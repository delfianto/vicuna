use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tui_markdown::from_str;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(area);

    // Session List (Left) - Placeholder
    let sessions_block = Block::default().borders(Borders::ALL).title("Sessions");
    f.render_widget(sessions_block, chunks[0]);

    // Chat Area (Right)
    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref()) // Input height 3
        .split(chunks[1]);

    let messages_area = chat_chunks[0];
    let input_area = chat_chunks[1];

    // Render Messages
    let history_md = app
        .messages
        .iter()
        .map(|m| format!("**{}**:\n{}", m.role.to_uppercase(), m.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let text = from_str(&history_md);
    let p = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title("Chat"));

    f.render_widget(p, messages_area);

    // Render Input

    f.render_widget(&app.input, input_area);
}
