use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::Paragraph,
    Frame, Terminal,
};
use std::io;
use tokio::sync::mpsc;

use crate::app::{Action, App, CurrentTab};
use crate::tui::components::toast;
use crate::tui::events::Event;

pub mod components;
pub mod events;
pub mod styles;
pub mod tabs;

pub fn init() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

pub async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    mut event_rx: mpsc::UnboundedReceiver<Event>,
    action_tx: mpsc::UnboundedSender<Action>,
) -> Result<()> {
    let _ = action_tx.send(Action::FetchModels);

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(f.area());

            let area = chunks[0];
            let bottom_area = chunks[1];

            match app.current_tab {
                CurrentTab::Models => tabs::models::draw(f, &app, area),
                CurrentTab::Chat => tabs::chat::draw(f, &app, area),
            }

            draw_help_bar(f, &app, bottom_area);

            if app.show_popup {
                app.popup.draw(f, area);
            }

            for t in &app.toasts {
                toast::draw(f, t, area);
            }
        })?;

        if let Some(event) = event_rx.recv().await {
            match event {
                Event::Input(key) => {
                    let actions = app.on_key(key);
                    for action in actions {
                        let _ = action_tx.send(action.clone());
                        if let Action::Quit = action {
                            return Ok(());
                        }
                    }
                }
                Event::ModelsFetched(models) => {
                    app.models = models;
                    app.sort_column = crate::app::SortColumn::Name;
                    app.sort_models();
                }
                Event::SessionsFetched(sessions) => {
                    app.sessions = sessions;
                }
                Event::MessagesLoaded(messages) => {
                    app.messages = messages
                        .into_iter()
                        .map(|(role, content)| crate::app::Message { role, content })
                        .collect();
                }
                Event::TokenReceived(token) => {
                    let should_append = if let Some(last) = app.messages.last() {
                        last.role == "assistant"
                    } else {
                        false
                    };

                    if should_append {
                        if let Some(last) = app.messages.last_mut() {
                            last.content.push_str(&token);
                        }
                    } else {
                        app.messages.push(crate::app::Message {
                            role: "assistant".into(),
                            content: token,
                        });
                    }
                }
                Event::GenerationDone => {
                    app.is_generating = false;
                    if let Some(last_msg) = app.messages.last()
                        && last_msg.role == "assistant"
                        && let Some(session_id) = &app.current_session_id
                    {
                        let _ = action_tx.send(Action::SaveMessage(
                            session_id.clone(),
                            "assistant".to_string(),
                            last_msg.content.clone(),
                        ));
                    }
                }
                Event::Error(msg) => {
                    app.show_error(&msg);
                }
                Event::Tick => {
                    app.on_tick();
                }
            }
        }
    }
}

fn draw_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_style = Style::default().fg(Color::Black).bg(Color::Cyan);
    let key_style = Style::default().fg(Color::DarkGray);
    let desc_style = Style::default().fg(Color::White);

    let mut spans = vec![
        Span::styled(format!(" {:?} ", app.current_tab), mode_style),
        Span::raw(" "),
    ];

    if app.current_tab == CurrentTab::Models
        && let Some(model) = app.models.get(app.selected_model_index)
    {
        spans.push(Span::styled(
            format!(" {} ", model.name),
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(40, 40, 40)),
        ));
        spans.push(Span::raw(" "));
    }

    let keys = match app.current_tab {
        CurrentTab::Models => vec![
            ("Tab", "Switch View"),
            ("j/k", "Nav"),
            ("Enter", "Chat"),
            ("s", "Sort"),
            ("p", "Pull"),
            ("d", "Delete"),
            ("Ctrl+q", "Quit"),
        ],
        CurrentTab::Chat => vec![
            ("Tab", "Switch View"),
            ("Enter", "Send/Load"),
            ("Ctrl+Arrows", "Switch Pane"),
            ("d", "Delete Session"),
            ("Ctrl+n", "New"),
            ("PgUp/Dn", "Scroll"),
            ("Ctrl+q", "Quit"),
        ],
    };

    for (key, desc) in keys {
        spans.push(Span::styled(format!("<{}>", key), key_style));
        spans.push(Span::styled(format!(" {} ", desc), desc_style));
    }

    let p = Paragraph::new(ratatui::text::Line::from(spans))
        .style(Style::default().bg(Color::Rgb(20, 20, 20)));
    f.render_widget(p, area);
}