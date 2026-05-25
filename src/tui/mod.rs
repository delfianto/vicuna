use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::Paragraph,
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
    mut event_rx: mpsc::Receiver<Event>,
    event_tx: mpsc::Sender<Event>,
    action_tx: mpsc::Sender<Action>,
) -> Result<()> {
    let _ = action_tx.send(Action::FetchModels).await;
    let _ = action_tx.send(Action::InitImage(60, 20)).await;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(f.area());

            let area = chunks[0];
            let status_area = chunks[1];
            let help_area = chunks[2];

            match app.current_tab {
                CurrentTab::Models => tabs::models::draw(f, &app, area),
                CurrentTab::Chat => tabs::chat::draw(f, &app, area),
            }

            draw_status_bar(f, &app, status_area);
            draw_help_bar(f, &app, help_area);

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
                        let _ = action_tx.send(action.clone()).await;
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
                    app.messages = messages;
                }
                Event::ModelInfoFetched(info) => {
                    app.model_info = Some(info);
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
                        app.messages.push(crate::db::repo::Message {
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
                        let _ = action_tx
                            .send(Action::SaveMessage(
                                crate::api::types::SessionId(session_id.clone()),
                                "assistant".to_string(),
                                last_msg.content.clone(),
                            ))
                            .await;
                    }
                }
                Event::Error(msg) => {
                    app.show_error(&msg);
                }
                Event::ImageLoaded(img) => {
                    let event_tx_img = event_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let protocol = ratatui_image::picker::Picker::from_termios().ok().and_then(
                            |mut picker| {
                                picker.guess_protocol();
                                picker
                                    .new_protocol(
                                        img,
                                        Rect::new(0, 0, 60, 20),
                                        ratatui_image::Resize::Fit(None),
                                    )
                                    .ok()
                            },
                        );
                        if let Some(p) = protocol {
                            let rt = tokio::runtime::Handle::current();
                            rt.block_on(async {
                                event_tx_img.send(Event::ImageInitialized(p)).await.ok();
                            });
                        }
                    });
                }
                Event::ImageInitialized(protocol) => {
                    app.logo = Some(protocol);
                }
                Event::Tick => {
                    app.on_tick();
                }
            }
        }
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_style = Style::default().fg(Color::Black).bg(Color::Cyan);
    let mut spans = vec![
        Span::styled(format!(" {:?} ", app.current_tab), mode_style),
        Span::raw(" "),
    ];

    if app.current_tab == CurrentTab::Models {
        if let Some(model) = app.models.get(app.selected_model_index) {
            spans.push(Span::styled(
                format!(" {} ", model.name),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    } else if app.current_tab == CurrentTab::Chat {
        if let Some(session_id) = &app.current_session_id {
            if let Some(session) = app.sessions.iter().find(|s| s.id.0 == *session_id) {
                spans.push(Span::styled(
                    format!(" {} ", session.title),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        } else {
            spans.push(Span::styled(
                " New Chat ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ));
        }
    }

    let p = Paragraph::new(ratatui::text::Line::from(spans))
        .style(Style::default().bg(Color::Rgb(30, 30, 30)));
    f.render_widget(p, area);
}

fn draw_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default().fg(Color::Cyan);
    let desc_style = Style::default().fg(Color::White);

    let mut spans = vec![];

    let keys = match app.current_tab {
        CurrentTab::Models => vec![
            ("Ctrl+D", "Chat"),
            ("Tab", "Pane"),
            ("j/k", "Nav"),
            ("Enter", "Chat"),
            ("s", "Sort"),
            ("i", "Info"),
            ("p", "Pull"),
            ("d", "Delete"),
            ("Ctrl+q", "Quit"),
        ],
        CurrentTab::Chat => vec![
            ("Ctrl+A", "Models"),
            ("Tab", "Pane"),
            ("Enter", "Send/Load"),
            ("Ctrl+Arrows", "Switch Pane"),
            ("d", "Delete Session"),
            ("Ctrl+n", "New"),
            ("PgUp/Dn", "Scroll"),
            ("Ctrl+q", "Quit"),
        ],
    };

    for (i, (key, desc)) in keys.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(format!("<{}>", key), key_style));
        spans.push(Span::styled(format!(" {} ", desc), desc_style));
    }

    let p = Paragraph::new(ratatui::text::Line::from(spans))
        .style(Style::default().bg(Color::Rgb(20, 20, 20)));
    f.render_widget(p, area);
}
