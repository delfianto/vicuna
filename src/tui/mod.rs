use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
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
    // Initial fetch
    let _ = action_tx.send(Action::FetchModels);

    loop {
        terminal.draw(|f| {
            let area = f.area();
            match app.current_tab {
                CurrentTab::Models => tabs::models::draw(f, &app, area),
                CurrentTab::Chat => tabs::chat::draw(f, &app, area),
            }

            for t in &app.toasts {
                toast::draw(f, t, area);
            }
        })?;

        if let Some(event) = event_rx.recv().await {
            match event {
                Event::Input(key) => {
                    if let Some(action) = app.on_key(key) {
                        let _ = action_tx.send(action.clone());
                        if let Action::Quit = action {
                            return Ok(());
                        }
                    }
                }
                Event::ModelsFetched(models) => {
                    app.models = models;
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
