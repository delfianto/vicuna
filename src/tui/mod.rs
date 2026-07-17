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
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::io;
use tokio::sync::mpsc;

use crate::app::{Action, App, ChatFocus, CurrentTab, ModelsFocus};
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
    _event_tx: mpsc::Sender<Event>,
    action_tx: mpsc::Sender<Action>,
) -> Result<()> {
    let _ = action_tx.send(Action::FetchModels).await;
    let _ = action_tx.send(Action::FetchSessions).await;

    loop {
        terminal.draw(|f| {
            // Top/side pad only — single chrome line flush to the bottom edge.
            let frame = styles::outer_area(f.area());

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .spacing(styles::GAP)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(frame);

            let area = chunks[0];
            let chrome_area = chunks[1];

            app.hits.clear();
            match app.current_tab {
                CurrentTab::Models => tabs::models::draw(f, &mut app, area),
                CurrentTab::Chat => tabs::chat::draw(f, &mut app, area),
            }

            draw_chrome_bar(f, &app, chrome_area);

            // Overlays use the full frame (not the padded content) so dim plates cover everything.
            let overlay_area = f.area();
            if app.show_popup {
                app.popup.draw(f, overlay_area);
            }
            if let Some(picker) = &app.model_picker {
                picker.draw(f, overlay_area, &app.models);
            }
            if let Some(confirm) = &app.confirm {
                confirm.draw(f, overlay_area);
            }

            for t in &app.toasts {
                toast::draw(f, t, overlay_area);
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
                Event::Mouse(mouse) => {
                    let actions = app.on_mouse(mouse);
                    for action in actions {
                        let _ = action_tx.send(action.clone()).await;
                    }
                }
                Event::ModelsFetched(models) => {
                    app.models = models;
                    app.sort_column = crate::app::SortColumn::Name;
                    app.sort_models();
                }
                Event::SessionsFetched(sessions) => {
                    app.sessions = sessions;
                    app.select_current_session();
                }
                Event::MessagesLoaded(messages) => {
                    app.messages = messages;
                    app.stick_chat_bottom();
                }
                Event::ModelInfoFetched(info) => {
                    app.model_info = Some(info);
                    // Keep inspect focused when payload arrives.
                    if app.show_info {
                        app.models_focus = ModelsFocus::Info;
                    }
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
                    // Stick to bottom while streaming (unless user scrolled up).
                    if app.chat_follow {
                        app.stick_chat_bottom();
                    }
                }
                Event::GenerationDone => {
                    app.is_generating = false;
                    if app.generation_cancelled {
                        // User aborted — keep partial text on screen, do not persist.
                        app.generation_cancelled = false;
                    } else if let Some(session_id) = app.current_session_id.clone() {
                        if let Some(last_msg) = app.messages.last()
                            && last_msg.role == "assistant"
                        {
                            let _ = action_tx
                                .send(Action::SaveMessage(
                                    crate::api::types::SessionId(session_id.clone()),
                                    "assistant".to_string(),
                                    last_msg.content.clone(),
                                ))
                                .await;
                        }
                        // Refine list title from the first user turn once the reply is done.
                        if let Some(user_prompt) =
                            app.messages.iter().find(|m| m.role == "user")
                        {
                            let title = App::final_session_title(&user_prompt.content);
                            app.rename_session_local(&session_id, title.clone());
                            let _ = action_tx
                                .send(Action::RenameSession(
                                    crate::api::types::SessionId(session_id),
                                    title,
                                ))
                                .await;
                        }
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

/// Single bottom chrome line: brand/tabs/context on the left, keys on the right.
fn draw_chrome_bar(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Paragraph::new("").style(styles::bar_bg()), area);

    let keys = current_help_keys(app);

    // Measure a reasonable keys budget: prefer ~55% of the row for bindings.
    let keys_budget = (area.width as usize * 55 / 100).max(20).min(area.width as usize);
    let help = styles::help_line(keys, keys_budget as u16);
    let help_w: u16 = help
        .spans
        .iter()
        .map(|s| s.content.chars().count() as u16)
        .sum::<u16>()
        .saturating_add(2) // " │ " separator space
        .min(area.width);

    let zones = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(help_w),
        ])
        .split(area);

    // ── Left: brand · tabs · session · mode ─────────────────────────────────
    let mut left: Vec<Span> = Vec::new();
    left.push(Span::styled(
        " vicuna ",
        Style::default()
            .fg(styles::BG_DEEP)
            .bg(styles::ACCENT)
            .add_modifier(Modifier::BOLD),
    ));
    left.push(Span::raw(" "));

    let models_active = app.current_tab == CurrentTab::Models;
    let chat_active = app.current_tab == CurrentTab::Chat;
    left.push(Span::styled(
        " models ",
        if models_active {
            styles::tab_active()
        } else {
            styles::tab_idle()
        },
    ));
    left.push(Span::raw(" "));
    left.push(Span::styled(
        " chat ",
        if chat_active {
            styles::tab_active()
        } else {
            styles::tab_idle()
        },
    ));

    // Session title only (model lives in the panel chrome).
    if app.current_tab == CurrentTab::Chat {
        left.push(Span::styled(" · ", styles::dim()));
        if let Some(session_id) = &app.current_session_id {
            let title = app
                .sessions
                .iter()
                .find(|s| s.id.0 == *session_id)
                .map(|s| {
                    if s.title.is_empty() {
                        "untitled".to_string()
                    } else {
                        s.title.clone()
                    }
                })
                .unwrap_or_else(|| "chat".into());
            let max = zones[0]
                .width
                .saturating_sub(
                    left.iter()
                        .map(|s| s.content.chars().count() as u16)
                        .sum::<u16>()
                        + 12,
                )
                .max(8) as usize;
            left.push(Span::styled(
                styles::short_model(&title, max),
                styles::text(),
            ));
        } else {
            left.push(Span::styled(
                "new chat",
                styles::muted().add_modifier(Modifier::ITALIC),
            ));
        }
    }

    // Mode / overlay badges
    left.push(Span::raw(" "));
    if app.confirm.is_some() {
        left.push(Span::styled(
            " CONFIRM ",
            Style::default()
                .fg(styles::BG_DEEP)
                .bg(styles::ERR)
                .add_modifier(Modifier::BOLD),
        ));
    } else if app.model_picker.is_some() {
        left.push(Span::styled(
            " PICKER ",
            Style::default()
                .fg(styles::BG_DEEP)
                .bg(styles::ACCENT)
                .add_modifier(Modifier::BOLD),
        ));
    } else if app.show_popup {
        left.push(Span::styled(
            " PULL ",
            Style::default()
                .fg(styles::BG_DEEP)
                .bg(styles::ACCENT)
                .add_modifier(Modifier::BOLD),
        ));
    } else if app.current_tab == CurrentTab::Chat {
        let (label, style) = match app.chat_focus {
            ChatFocus::Input => (
                " INS ",
                Style::default()
                    .fg(styles::BG_DEEP)
                    .bg(styles::OK)
                    .add_modifier(Modifier::BOLD),
            ),
            ChatFocus::Conversation => (
                " CHAT ",
                Style::default()
                    .fg(styles::BG_DEEP)
                    .bg(styles::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            ChatFocus::Sessions => (
                " LIST ",
                Style::default()
                    .fg(styles::BG_DEEP)
                    .bg(styles::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
        };
        left.push(Span::styled(label, style));
    }

    if app.is_generating {
        left.push(Span::styled(
            format!(" {} ", app.spinner_glyph()),
            Style::default()
                .fg(styles::OK)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // ── Right: key hints ────────────────────────────────────────────────────
    let mut right = vec![Span::styled("│ ", styles::dim())];
    right.extend(help.spans);

    f.render_widget(
        Paragraph::new(Line::from(left)).style(styles::bar_bg()),
        zones[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(right)).style(styles::bar_bg()),
        zones[1],
    );
}

fn current_help_keys(app: &App) -> &'static [(&'static str, &'static str)] {
    if app.confirm.is_some() {
        &[("y", "confirm"), ("n/esc", "cancel")]
    } else if app.model_picker.is_some() {
        &[
            ("j/k", "nav"),
            ("enter", "use"),
            ("type", "filter"),
            ("esc", "close"),
        ]
    } else if app.show_popup {
        &[("enter", "pull"), ("esc", "cancel")]
    } else {
        match app.current_tab {
            CurrentTab::Models => {
                if app.show_info {
                    match app.models_focus {
                        ModelsFocus::Info => &[
                            ("↑↓", "scroll"),
                            ("f3/esc", "close"),
                            ("tab", "list"),
                            ("j/k", "scroll"),
                        ],
                        ModelsFocus::List => &[
                            ("f3/esc", "close"),
                            ("tab", "inspect"),
                            ("j/k", "nav"),
                            ("enter", "chat"),
                            ("p", "pull"),
                        ],
                    }
                } else {
                    &[
                        ("f3", "inspect"),
                        ("esc", "chat"),
                        ("j/k", "nav"),
                        ("enter", "use"),
                        ("p", "pull"),
                        ("d", "delete"),
                        ("s", "sort"),
                    ]
                }
            }
            CurrentTab::Chat => match app.chat_focus {
                ChatFocus::Sessions => {
                    if app.is_generating {
                        &[("c", "cancel"), ("tab", "focus"), ("j/k", "sessions")]
                    } else {
                        &[
                            ("tab", "focus"),
                            ("j/k", "sessions"),
                            ("enter", "open"),
                            ("i", "insert"),
                            ("f2", "library"),
                            ("m", "model"),
                        ]
                    }
                }
                ChatFocus::Conversation => {
                    if app.is_generating {
                        &[("↑↓", "scroll"), ("tab", "focus"), ("^c", "cancel")]
                    } else {
                        &[
                            ("↑↓/jk", "scroll"),
                            ("tab", "focus"),
                            ("i", "insert"),
                            ("G", "bottom"),
                            ("r", "regen"),
                        ]
                    }
                }
                ChatFocus::Input => {
                    if app.is_generating {
                        &[("^c", "cancel"), ("tab", "focus"), ("esc", "back")]
                    } else {
                        &[
                            ("enter", "send"),
                            ("tab", "focus"),
                            ("esc", "back"),
                            ("f2", "library"),
                            ("^r", "regen"),
                            ("^c^c", "quit"),
                        ]
                    }
                }
            },
        }
    }
}
