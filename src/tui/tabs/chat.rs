use crate::app::{App, ChatFocus};
use crate::tui::components::markdown::MarkdownViewer;
use crate::tui::styles;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(area);

    // Session List (Left)
    let sessions: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, (_id, title, model, _date))| {
            let display_title = if title.is_empty() { "Untitled" } else { title };
            let display = format!("{} [{}]", display_title, model);
            ListItem::new(display).style(styles::get_rainbow_style(i))
        })
        .collect();

    let (session_border_style, session_border_type) = if app.chat_focus == ChatFocus::Sessions {
        (Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD), BorderType::Thick)
    } else {
        (Style::default().fg(Color::DarkGray), BorderType::Plain)
    };

    let sessions_list = List::new(sessions)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(session_border_type)
                .title("Sessions")
                .border_style(session_border_style)
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(" ➤ ");

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.selected_session_index));
    
    f.render_stateful_widget(sessions_list, chunks[0], &mut state);

    // Chat Area (Right)
    let input_lines = app.input.lines().len() as u16;
    let input_height = (input_lines + 2).clamp(3, 10);

    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(input_height)].as_ref()) 
        .split(chunks[1]);

    let messages_area = chat_chunks[0];
    let input_area = chat_chunks[1];

    let model_name = app.models.get(app.selected_model_index)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // Render Messages
    let history_md = app
        .messages
        .iter()
        .map(|m| format!("**{}**:\n{}", m.role.to_uppercase(), m.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let chat_title = format!(" Chat with {} ", model_name);
    
    // Gradient-like effect for Chat border based on active model index?
    // Let's just use a fixed nice color, or rainbow based on model index.
    let chat_border_color = styles::RAINBOW[app.selected_model_index % styles::RAINBOW.len()];
    
    let viewer = MarkdownViewer::new(&history_md)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(ratatui::text::Span::styled(
                chat_title,
                Style::default().fg(chat_border_color).add_modifier(Modifier::BOLD)
            ))
            .border_style(Style::default().fg(chat_border_color))
        )
        .scroll(app.chat_scroll);

    f.render_widget(viewer, messages_area);

    // Render Input
    let (input_border_style, input_border_type) = if app.chat_focus == ChatFocus::Input {
        (Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD), BorderType::Thick)
    } else {
        (Style::default().fg(Color::DarkGray), BorderType::Plain)
    };
    
    // Create a clone of the input to modify its block style for rendering
    let mut input = app.input.clone();
    input.set_block(
        ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(input_border_type)
            .title("Input")
            .border_style(input_border_style)
    );
    
    f.render_widget(&input, input_area);

    // Render input scrollbar if content is larger than input area
    if input_lines > input_height.saturating_sub(2) {
        let scrollbar = ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(input_lines as usize)
            .position(app.input.cursor().0);
        f.render_stateful_widget(
            scrollbar,
            input_area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}