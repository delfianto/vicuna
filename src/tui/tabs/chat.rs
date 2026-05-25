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

    let sessions: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let display_title = if session.title.is_empty() {
                "Untitled"
            } else {
                &session.title
            };
            let display = format!("{} [{}]", display_title, session.model);
            ListItem::new(display).style(styles::get_rainbow_style(i))
        })
        .collect();

    let (session_border_style, session_border_type) = if app.chat_focus == ChatFocus::Sessions {
        (
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
            BorderType::Thick,
        )
    } else {
        (Style::default().fg(Color::DarkGray), BorderType::Plain)
    };

    let sessions_list = List::new(sessions)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(session_border_type)
                .title(" Sessions ")
                .border_style(session_border_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ➤ ");

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.selected_session_index));

    f.render_stateful_widget(sessions_list, chunks[0], &mut state);

    let estimated_width = chunks[1].width.saturating_sub(4).max(1);
    let mut visual_input_lines: u16 = 0;
    for line in app.input.lines() {
        let s: &str = line.as_str();
        let len = s.chars().count() as u16;
        if len == 0 {
            visual_input_lines += 1;
        } else {
            visual_input_lines += len.div_ceil(estimated_width);
        }
    }
    let input_height = (visual_input_lines + 2).clamp(3, 15);

    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(input_height)].as_ref())
        .split(chunks[1]);

    let messages_area = chat_chunks[0];
    let input_area = chat_chunks[1];

    if app.messages.is_empty() {
        draw_empty_chat(f, app, messages_area);
    } else {
        let model_name = app
            .models
            .get(app.selected_model_index)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let history_md = app
            .messages
            .iter()
            .map(|m| format!("**{}**:\n{}", m.role.to_uppercase(), m.content))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        let chat_title = format!(" Chat with {} ", model_name);

        let chat_border_color = styles::RAINBOW[app.selected_model_index % styles::RAINBOW.len()];

        let viewer = MarkdownViewer::new(&history_md)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(ratatui::text::Span::styled(
                        chat_title,
                        Style::default()
                            .fg(chat_border_color)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_style(Style::default().fg(chat_border_color)),
            )
            .scroll(app.chat_scroll);

        f.render_widget(viewer, messages_area);
    }

    let (input_border_style, input_border_type) = if app.chat_focus == ChatFocus::Input {
        (
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
            BorderType::Thick,
        )
    } else {
        (Style::default().fg(Color::DarkGray), BorderType::Plain)
    };

    let mut input = app.input.clone();
    input.set_block(
        ratatui::widgets::Block::default()
            .borders(Borders::ALL)
            .border_type(input_border_type)
            .title(" Input ")
            .border_style(input_border_style),
    );

    f.render_widget(&input, input_area);

    if visual_input_lines > input_height.saturating_sub(2) {
        let scrollbar =
            ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));
        let mut scrollbar_state =
            ratatui::widgets::ScrollbarState::new(visual_input_lines as usize)
                .position(app.input.cursor().0);
        f.render_stateful_widget(
            scrollbar,
            input_area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn draw_empty_chat(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let model_name = app
        .models
        .get(app.selected_model_index)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let chat_border_color = styles::RAINBOW[app.selected_model_index % styles::RAINBOW.len()];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(ratatui::text::Span::styled(
            format!(" New Chat with {} ", model_name),
            Style::default()
                .fg(chat_border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(chat_border_color));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let center_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(20),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(inner_area);

    if let Some(logo) = &app.logo {
        let image_area = center_layout[1];
        let image_width = 60;
        let horizontal_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - (image_width * 100 / inner_area.width.max(1))) / 2),
                Constraint::Length(image_width),
                Constraint::Percentage((100 - (image_width * 100 / inner_area.width.max(1))) / 2),
            ])
            .split(image_area);

        let image_widget = ratatui_image::Image::new(logo.as_ref());
        f.render_widget(image_widget, horizontal_layout[1]);
    }

    let instructions = ratatui::text::Line::from(vec![
        ratatui::text::Span::raw("Type a message to start chatting with "),
        ratatui::text::Span::styled(
            &model_name,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let inst_p = ratatui::widgets::Paragraph::new(instructions)
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(inst_p, center_layout[2]);
}
