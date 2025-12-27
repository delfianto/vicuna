use crate::api::client::OllamaClient;
use crate::app::Action;
use crate::db::repo::Repository;
use crate::tui::events::Event;
use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct Backend {
    client: OllamaClient,
    repo: Arc<Repository>,
    action_rx: UnboundedReceiver<Action>,
    event_tx: UnboundedSender<Event>,
    generation_task: Option<tokio::task::AbortHandle>,
}

impl Backend {
    pub fn new(
        client: OllamaClient,
        repo: Arc<Repository>,
        action_rx: UnboundedReceiver<Action>,
        event_tx: UnboundedSender<Event>,
    ) -> Self {
        Self {
            client,
            repo,
            action_rx,
            event_tx,
            generation_task: None,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        while let Some(action) = self.action_rx.recv().await {
            match action {
                Action::Quit => break,
                Action::FetchModels => {
                    let client = self.client.clone();
                    let event_tx = self.event_tx.clone();
                    let repo = self.repo.clone();
                    tokio::spawn(async move {
                        match client.list_models().await {
                            Ok(res) => {
                                for model in &res.models {
                                    if let Err(e) =
                                        crate::db::repo::upsert_model(&repo.conn, model).await
                                    {
                                        tracing::error!(
                                            "Failed to upsert model {}: {:?}",
                                            model.name,
                                            e
                                        );
                                    }
                                }
                                event_tx.send(Event::ModelsFetched(res.models)).ok();
                            }
                            Err(e) => {
                                event_tx
                                    .send(Event::Error(format!("Failed to fetch models: {}", e)))
                                    .ok();
                            }
                        }
                    });
                }
                Action::ShowModelInfo(name) => {
                    let client = self.client.clone();
                    let event_tx = self.event_tx.clone();
                    tokio::spawn(async move {
                        match client.show_model(&name.0).await {
                            Ok(res) => {
                                event_tx.send(Event::ModelInfoFetched(res)).ok();
                            }
                            Err(e) => {
                                event_tx
                                    .send(Event::Error(format!(
                                        "Failed to fetch model info: {}",
                                        e
                                    )))
                                    .ok();
                            }
                        }
                    });
                }
                Action::DeleteModel(name) => {
                    let client = self.client.clone();
                    let event_tx = self.event_tx.clone();
                    let repo = self.repo.clone();
                    tokio::spawn(async move {
                        match client.delete_model(&name.0).await {
                            Ok(success) => {
                                if success {
                                    if let Err(e) =
                                        crate::db::repo::delete_model_cascade(&repo.conn, &name.0)
                                            .await
                                    {
                                        tracing::error!("Failed to delete model from DB: {:?}", e);
                                    }
                                    event_tx
                                        .send(Event::Error("Model deleted".to_string()))
                                        .ok();
                                } else {
                                    event_tx
                                        .send(Event::Error("Failed to delete model".to_string()))
                                        .ok();
                                }
                            }
                            Err(e) => {
                                event_tx
                                    .send(Event::Error(format!("Failed to delete model: {}", e)))
                                    .ok();
                            }
                        }
                    });
                }
                Action::PullModel(name) => {
                    let client = self.client.clone();
                    let event_tx = self.event_tx.clone();
                    let req = crate::api::types::PullRequest {
                        name: name.0.clone(),
                        stream: Some(true),
                    };
                    tokio::spawn(async move {
                        let stream = client.pull_model_stream(req);
                        let mut stream = Box::pin(stream);
                        while let Some(res) = stream.next().await {
                            match res {
                                Ok(resp) => {
                                    tracing::debug!("Pull status: {}", resp.status);
                                }
                                Err(e) => {
                                    event_tx
                                        .send(Event::Error(format!("Pull error: {}", e)))
                                        .ok();
                                    return;
                                }
                            }
                        }
                        event_tx
                            .send(Event::Error("Model pulled successfully".to_string()))
                            .ok();
                    });
                }
                Action::Generate(prompt, model) => {
                    if let Some(handle) = self.generation_task.take() {
                        handle.abort();
                    }

                    let client = self.client.clone();
                    let event_tx = self.event_tx.clone();

                    let req = crate::api::types::GenerateRequest {
                        model: model.0.clone(),
                        prompt: prompt.clone(),
                        stream: Some(true),
                        system: None,
                        template: None,
                        context: None,
                    };

                    let handle = tokio::spawn(async move {
                        let stream = client.generate_stream(req);
                        let mut stream = Box::pin(stream);

                        while let Some(res) = stream.next().await {
                            match res {
                                Ok(resp) => {
                                    event_tx.send(Event::TokenReceived(resp.response)).ok();
                                    if resp.done {
                                        event_tx.send(Event::GenerationDone).ok();
                                    }
                                }
                                Err(e) => {
                                    event_tx
                                        .send(Event::Error(format!("Generation error: {}", e)))
                                        .ok();
                                    event_tx.send(Event::GenerationDone).ok();
                                }
                            }
                        }
                    });
                    self.generation_task = Some(handle.abort_handle());
                }
                Action::FetchSessions => {
                    let repo = self.repo.clone();
                    let event_tx = self.event_tx.clone();
                    tokio::spawn(async move {
                        match crate::db::repo::get_sessions(&repo.conn).await {
                            Ok(sessions) => {
                                event_tx.send(Event::SessionsFetched(sessions)).ok();
                            }
                            Err(e) => {
                                event_tx
                                    .send(Event::Error(format!("Failed to fetch sessions: {}", e)))
                                    .ok();
                            }
                        }
                    });
                }
                Action::LoadSession(session_id) => {
                    let repo = self.repo.clone();
                    let event_tx = self.event_tx.clone();
                    tokio::spawn(async move {
                        match crate::db::repo::get_messages(&repo.conn, &session_id.0).await {
                            Ok(messages) => {
                                event_tx.send(Event::MessagesLoaded(messages)).ok();
                            }
                            Err(e) => {
                                event_tx
                                    .send(Event::Error(format!("Failed to load messages: {}", e)))
                                    .ok();
                            }
                        }
                    });
                }
                Action::CreateSession(id, title, model) => {
                    let repo = self.repo.clone();
                    let event_tx = self.event_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            crate::db::repo::create_session(&repo.conn, &id.0, &title, &model.0)
                                .await
                        {
                            event_tx
                                .send(Event::Error(format!("Failed to create session: {}", e)))
                                .ok();
                        }
                    });
                }
                Action::DeleteSession(id) => {
                    let repo = self.repo.clone();
                    let event_tx = self.event_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = crate::db::repo::delete_session(&repo.conn, &id.0).await {
                            event_tx
                                .send(Event::Error(format!("Failed to delete session: {}", e)))
                                .ok();
                        }
                    });
                }
                Action::SaveMessage(session_id, role, content) => {
                    let repo = self.repo.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            crate::db::repo::add_message(&repo.conn, &session_id.0, &role, &content)
                                .await
                        {
                            tracing::error!("Failed to save message: {:?}", e);
                        }
                    });
                }
                Action::InitImage(w, h) => {
                    let event_tx = self.event_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        #[derive(rust_embed::RustEmbed)]
                        #[folder = "asset/"]
                        struct Asset;

                        if let Some(file) = Asset::get("vicuna-logo.png")
                            && let Ok(img) = image::load_from_memory(&file.data)
                        {
                            let fixed = img.resize_exact(
                                w as u32,
                                h as u32,
                                image::imageops::FilterType::Lanczos3,
                            );
                            event_tx.send(Event::ImageLoaded(fixed)).ok();
                        }
                    });
                }
            }
        }
        Ok(())
    }
}
