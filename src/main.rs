mod api;
mod app;
mod backend;
mod config;
mod db;
mod logging;
mod tui;
mod utils;

use crate::api::client::OllamaClient;
use crate::db::repo::Repository;
use anyhow::Result;
use app::{Action, App};
use backend::Backend;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::Config::new()?;

    let _guard = logging::init(&config.log_dir)?;
    tracing::info!("Vicuna starting up...");

    let repo = Arc::new(Repository::new(&config.db_path).await?);
    let client = OllamaClient::new("http://localhost:11434".to_string());

    let (action_tx, action_rx) = mpsc::unbounded_channel::<Action>();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    let event_tx_input = event_tx.clone();
    tokio::spawn(async move {
        let mut reader = crossterm::event::EventStream::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        loop {
            tokio::select! {
                Some(Ok(evt)) = reader.next() => {
                    if let crossterm::event::Event::Key(key) = evt {
                        event_tx_input.send(crate::tui::events::Event::Input(key)).ok();
                    }
                }
                _ = interval.tick() => {
                    event_tx_input.send(crate::tui::events::Event::Tick).ok();
                }
            }
        }
    });

    let backend = Backend::new(client, repo, action_rx, event_tx.clone());
    tokio::spawn(async move {
        if let Err(e) = backend.run().await {
            tracing::error!("Backend error: {}", e);
        }
    });

    let mut terminal = tui::init()?;
    let app = App::new(config.clone());

    // Initial actions
    action_tx.send(Action::FetchModels).ok();
    action_tx.send(Action::FetchSessions).ok();

    let res = tui::run_app(&mut terminal, app, event_rx, event_tx, action_tx).await;

    tui::restore()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
