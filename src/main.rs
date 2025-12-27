mod api;
mod app;
mod config;
mod db;
mod logging;
mod tui;
mod utils;

use anyhow::Result;
use app::{Action, App};
use crossterm::event::{self, Event as CEvent};
use futures::StreamExt;
use std::time::Duration;
use tui::events::Event;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::Config::new()?;

    let _guard = logging::init(&config.log_dir)?;
    tracing::info!("Vicuna starting up...");

    let _conn = db::init(&config.db_path).await?;
    tracing::info!("Database initialized at {:?}", config.db_path);

    let mut terminal = tui::init()?;

    let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel();
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

    let event_tx_input = event_tx.clone();
    std::thread::spawn(move || {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = std::time::Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("no events")
                && let CEvent::Key(key) = event::read().expect("can't read events")
            {
                event_tx_input.send(Event::Input(key)).ok();
            }
            if last_tick.elapsed() >= tick_rate {
                event_tx_input.send(Event::Tick).ok();
                last_tick = std::time::Instant::now();
            }
        }
    });

    let event_tx_backend = event_tx.clone();
    let action_tx_backend = action_tx.clone();
    let client = api::client::OllamaClient::new("http://localhost:11434".to_string());

    tokio::spawn(async move {
        while let Some(action) = action_rx.recv().await {
            match action {
                Action::FetchModels => match client.list_models().await {
                    Ok(resp) => {
                        event_tx_backend
                            .send(Event::ModelsFetched(resp.models))
                            .ok();
                    }
                    Err(e) => {
                        tracing::error!("Failed to list models: {:?}", e);
                        event_tx_backend
                            .send(Event::Error(format!("Failed to list models: {}", e)))
                            .ok();
                    }
                },
                Action::Quit => {
                    break;
                }
                Action::DeleteModel(name) => match client.delete_model(&name).await {
                    Ok(_) => {
                        action_tx_backend.send(Action::FetchModels).ok();
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete model: {:?}", e);
                        action_tx_backend.send(Action::FetchModels).ok();
                    }
                },
                Action::PullModel(name) => {
                    tracing::info!("Pulling model {}", name);
                }
                Action::Generate(prompt, model) => {
                    let req = api::types::GenerateRequest {
                        model,
                        prompt,
                        system: None,
                        template: None,
                        context: None,
                        stream: Some(true),
                    };

                    let stream = client.generate_stream(req);
                    let event_tx_gen = event_tx_backend.clone();

                    tokio::spawn(async move {
                        let mut stream = Box::pin(stream);
                        while let Some(res) = stream.next().await {
                            match res {
                                Ok(resp) => {
                                    event_tx_gen.send(Event::TokenReceived(resp.response)).ok();
                                    if resp.done {
                                        event_tx_gen.send(Event::GenerationDone).ok();
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Generation error: {:?}", e);
                                    event_tx_gen
                                        .send(Event::Error(format!("Generation error: {}", e)))
                                        .ok();
                                    event_tx_gen.send(Event::GenerationDone).ok();
                                }
                            }
                        }
                    });
                }
            }
        }
    });

    let app = App::new(config.clone());

    let res = tui::run_app(&mut terminal, app, event_rx, action_tx).await;

    tui::restore()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
