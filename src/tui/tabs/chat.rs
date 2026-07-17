use crate::app::{App, ChatFocus};
use crate::tui::components::markdown::MarkdownViewer;
use crate::tui::styles;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    // Full-width composer under a side-by-side top row so bottom edges align:
    //   ┌ sessions ┬ conversation ┐
    //   │          │              │
    //   ├──────────┴──────────────┤
    //   │ composer                │
    //   └─────────────────────────┘
    let estimated_width = area.width.saturating_sub(6).max(1);
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
    let input_height = (visual_input_lines + 3).clamp(4, 12);

    let (top_area, input_area) =
        styles::split_vertical(area, Constraint::Min(1), Constraint::Length(input_height));

    let (sessions_area, messages_area) =
        styles::split_horizontal(top_area, Constraint::Length(28), Constraint::Min(20));

    app.hits.sessions = Some(sessions_area);
    app.hits.messages = Some(messages_area);
    app.hits.composer = Some(input_area);
    app.hits.models_list = None;
    app.hits.models_info = None;

    draw_sessions(f, app, sessions_area);

    if app.messages.is_empty() {
        app.chat_max_scroll = 0;
        if app.chat_follow {
            app.chat_scroll = 0;
        }
        draw_empty_chat(f, app, messages_area);
    } else {
        draw_messages(f, app, messages_area);
    }

    draw_composer(f, app, input_area, visual_input_lines, input_height);
}

fn draw_sessions(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.chat_focus == ChatFocus::Sessions;

    let sessions: Vec<ListItem> = if app.sessions.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  no sessions yet",
            styles::muted().add_modifier(Modifier::ITALIC),
        )))]
    } else {
        app.sessions
            .iter()
            .map(|session| {
                let title = if session.title.is_empty() {
                    "untitled"
                } else {
                    session.title.as_str()
                };
                let line = Line::from(vec![
                    Span::styled(format!("{title}  "), styles::text()),
                    Span::styled(truncate_model(&session.model.0, 14), styles::dim()),
                ]);
                ListItem::new(line)
            })
            .collect()
    };

    let count = app.sessions.len();
    let title = if count == 0 {
        "sessions".to_string()
    } else {
        format!("sessions · {count}")
    };

    let sessions_list = List::new(sessions)
        .block(styles::pane_block(title, focused))
        .highlight_style(styles::HIGHLIGHT_STYLE)
        .highlight_symbol("▎ ");

    let mut state = ratatui::widgets::ListState::default();
    if !app.sessions.is_empty() {
        state.select(Some(app.selected_session_index));
    }

    f.render_stateful_widget(sessions_list, area, &mut state);
}

fn draw_messages(f: &mut Frame, app: &mut App, area: Rect) {
    let model_name = app
        .models
        .get(app.selected_model_index)
        .map(|m| m.name.as_str())
        .unwrap_or("?")
        .to_string();

    let history_md = format_transcript(&app.messages, app.is_generating);

    let focused = app.chat_focus == ChatFocus::Conversation;
    let title = if app.is_generating {
        format!(
            "conversation · {model_name} · {} streaming",
            app.spinner_glyph()
        )
    } else if focused {
        format!("conversation · {model_name} · ↑↓ scroll")
    } else {
        format!("conversation · {model_name}")
    };

    let block = styles::pane_block(title, focused);
    let inner = block.inner(area);

    // Keep scroll state aligned with what the viewer will actually use.
    let max_scroll = MarkdownViewer::max_scroll_for(&history_md, inner.width, inner.height);
    app.chat_max_scroll = max_scroll;
    if app.chat_follow {
        app.chat_scroll = max_scroll;
    } else {
        app.chat_scroll = app.chat_scroll.min(max_scroll);
    }

    let viewer = MarkdownViewer::new(&history_md)
        .block(block)
        .scroll(app.chat_scroll)
        .follow(app.chat_follow);

    f.render_widget(viewer, area);
}

/// Build markdown that `tui-markdown` can render cleanly (roles, body, code fences).
fn format_transcript(messages: &[crate::db::repo::Message], is_generating: bool) -> String {
    let mut out = String::new();
    for (i, m) in messages.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n---\n\n");
        }
        let label = styles::role_label(&m.role);
        // h3 keeps role labels distinct without eating half the pane like h1.
        out.push_str(&format!("### {label}\n\n"));
        // Normalize content: ensure fenced code blocks have blank lines around them
        // when models forget, so the parser doesn't swallow following prose.
        out.push_str(&normalize_md_body(m.content.trim_end()));
        if is_generating && i + 1 == messages.len() && m.role == "assistant" {
            out.push_str(" ▍");
        }
    }
    out
}

fn normalize_md_body(content: &str) -> String {
    // Light touch: collapse 3+ blank lines, keep code fences intact.
    let mut result = String::with_capacity(content.len());
    let mut blank_run = 0u8;
    for line in content.lines() {
        if line.trim().is_empty() {
            blank_run = blank_run.saturating_add(1);
            if blank_run <= 2 {
                result.push('\n');
            }
        } else {
            blank_run = 0;
            result.push_str(line);
            result.push('\n');
        }
    }
    // Trim trailing newline added by loop if original had none? Prefer stable trailing \n.
    result
}

fn draw_composer(f: &mut Frame, app: &App, area: Rect, visual_input_lines: u16, input_height: u16) {
    let focused = app.chat_focus == ChatFocus::Input;

    let title = if app.is_generating {
        format!("composer · {} generating · ^c cancel", app.spinner_glyph())
    } else if focused {
        "composer · enter send · tab cycle · esc back".to_string()
    } else {
        "composer · i / tab to focus".to_string()
    };

    let mut input = app.input.clone();
    input.set_block(styles::pane_block(title, focused));
    input.set_cursor_line_style(Style::default());

    f.render_widget(&input, area);

    let viewport_h = input_height.saturating_sub(2).max(1);
    if visual_input_lines > viewport_h {
        // Cursor row is a line index (0..lines-1) — selection-style scrollbar.
        // viewport_content_length keeps thumb size proportional to visible rows.
        let scrollbar =
            ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));
        let cursor_row = app
            .input
            .cursor()
            .0
            .min(visual_input_lines.saturating_sub(1) as usize);
        let mut scrollbar_state =
            ratatui::widgets::ScrollbarState::new(visual_input_lines as usize)
                .position(cursor_row)
                .viewport_content_length(viewport_h as usize);
        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn draw_empty_chat(f: &mut Frame, app: &App, area: Rect) {
    let model_name = app
        .models
        .get(app.selected_model_index)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "no model selected".to_string());

    let block = styles::pane_block(format!("conversation · {model_name}"), false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Compact alpaca / vicuna-ish glyph — no image protocol noise.
    let art = [
        r"      .--.  ",
        r"     / .. \/ ",
        r"    ( \  / / ",
        r"     \ \/ /  ",
        r"   __|  | /  ",
        r"  /   \_/|   ",
        r"  \  /  \|   ",
        r"   \/ /\  \  ",
        r"    / /  \ | ",
        r"   / /   | | ",
    ];

    let mut lines: Vec<Line> = Vec::new();
    if inner.height >= 14 {
        for row in art {
            lines.push(Line::from(Span::styled(
                row.to_string(),
                styles::accent().add_modifier(Modifier::DIM),
            )));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "start a conversation",
        styles::accent_bold(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("type below · ", styles::muted()),
        Span::styled(model_name, styles::accent_bold()),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" esc ", styles::key_cap()),
        Span::styled(" back  ", styles::key_desc()),
        Span::styled(" m ", styles::key_cap()),
        Span::styled(" model  ", styles::key_desc()),
        Span::styled(" f2 ", styles::key_cap()),
        Span::styled(" library", styles::key_desc()),
    ]));

    let body_h = lines.len() as u16;
    let center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(body_h.min(inner.height)),
            Constraint::Min(1),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(lines).alignment(ratatui::layout::HorizontalAlignment::Center),
        center[1],
    );
}

fn truncate_model(name: &str, max: usize) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= max {
        name.to_string()
    } else if max <= 1 {
        "…".to_string()
    } else {
        let mut s: String = chars.into_iter().take(max.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}
