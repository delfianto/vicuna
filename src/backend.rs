use crate::api::client::OllamaClient;
use crate::api::types::{GenerateRequest, ModelName, SessionId};
use crate::app::Action;
use crate::assets::Asset;
use crate::db::repo::Repository;
use crate::tui::events::Event;
use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct Backend {
    client: OllamaClient,
    repo: Arc<Repository>,
    action_rx: Receiver<Action>,
    event_tx: Sender<Event>,
    generation_task: Option<tokio::task::AbortHandle>,
}

impl Backend {
    pub fn new(
        client: OllamaClient,
        repo: Arc<Repository>,
        action_rx: Receiver<Action>,
        event_tx: Sender<Event>,
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
                Action::InitImage(w, h) => self.handle_init_image(w, h),
                Action::FetchModels => self.handle_fetch_models(),
                Action::FetchSessions => self.handle_fetch_sessions(),
                Action::ShowModelInfo(name) => self.handle_show_model_info(name),
                Action::DeleteModel(name) => self.handle_delete_model(name),
                Action::PullModel(name) => self.handle_pull_model(name),
                Action::Generate(prompt, model) => self.handle_generate(prompt, model),
                Action::LoadSession(id) => self.handle_load_session(id),
                Action::CreateSession(id, title, model) => {
                    self.handle_create_session(id, title, model)
                }
                Action::DeleteSession(id) => self.handle_delete_session(id),
                Action::SaveMessage(id, role, content) => {
                    self.handle_save_message(id, role, content)
                }
            }
        }
        Ok(())
    }

    fn handle_init_image(&self, w: u16, h: u16) {
        let event_tx = self.event_tx.clone();

        tokio::task::spawn_blocking(move || {
            let Some(file) = Asset::get("vicuna-logo.png") else {
                tracing::warn!("Asset 'vicuna-logo.png' not found");
                return;
            };

            let Ok(img) = image::load_from_memory(&file.data) else {
                tracing::error!("Failed to decode vicuna-logo.png");
                return;
            };

            let fixed = img.resize_exact(w as u32, h as u32, image::imageops::FilterType::Lanczos3);

            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                if let Err(e) = event_tx.send(Event::ImageLoaded(fixed)).await {
                    tracing::error!("Failed to send image event: {}", e);
                }
            });
        });
    }

    fn handle_fetch_models(&self) {
        let client = self.client.clone();
        let repo = self.repo.clone();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            match client.list_models().await {
                Ok(res) => {
                    for model in &res.models {
                        if let Err(e) = crate::db::repo::upsert_model(&repo.conn, model).await {
                            tracing::error!("DB Upsert failed for model {}: {}", model.name, e);
                        }
                    }
                    tx.send(Event::ModelsFetched(res.models)).await.ok();
                }
                Err(e) => {
                    tx.send(Event::Error(format!("Fetch models failed: {}", e)))
                        .await
                        .ok();
                }
            }
        });
    }

    fn handle_show_model_info(&self, name: ModelName) {
        let client = self.client.clone();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            match client.show_model(&name.0).await {
                Ok(res) => {
                    tx.send(Event::ModelInfoFetched(res)).await.ok();
                }
                Err(e) => {
                    tx.send(Event::Error(format!("Fetch model info failed: {}", e)))
                        .await
                        .ok();
                }
            }
        });
    }

    fn handle_delete_model(&self, name: ModelName) {
        let client = self.client.clone();
        let repo = self.repo.clone();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            match client.delete_model(&name.0).await {
                Ok(true) => {
                    if let Err(e) = crate::db::repo::delete_model_cascade(&repo.conn, &name).await {
                        tracing::error!("DB Cascade delete failed: {}", e);
                    }
                    tx.send(Event::Error(format!("Model {} deleted", name.0)))
                        .await
                        .ok();
                }
                Ok(false) => {
                    tx.send(Event::Error(format!(
                        "Model {} not found or not deleted",
                        name.0
                    )))
                    .await
                    .ok();
                }
                Err(e) => {
                    tx.send(Event::Error(format!("Delete failed: {}", e)))
                        .await
                        .ok();
                }
            }
        });
    }

    fn handle_pull_model(&self, name: ModelName) {
        let client = self.client.clone();
        let tx = self.event_tx.clone();
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
                        tracing::debug!("Pull status for {}: {}", name.0, resp.status);
                    }
                    Err(e) => {
                        tx.send(Event::Error(format!("Pull error for {}: {}", name.0, e)))
                            .await
                            .ok();
                        return;
                    }
                }
            }
            tx.send(Event::Error(format!(
                "Model {} pulled successfully",
                name.0
            )))
            .await
            .ok();
        });
    }

    fn handle_generate(&mut self, prompt: String, model: ModelName) {
        if let Some(handle) = self.generation_task.take() {
            handle.abort();
        }

        let client = self.client.clone();
        let tx = self.event_tx.clone();

        let req = GenerateRequest {
            model: model.0,
            prompt,
            stream: Some(true),
            ..Default::default()
        };

        let handle = tokio::spawn(async move {
            let mut stream = Box::pin(client.generate_stream(req));
            while let Some(res) = stream.next().await {
                match res {
                    Ok(resp) => {
                        tx.send(Event::TokenReceived(resp.response)).await.ok();
                        if resp.done {
                            tx.send(Event::GenerationDone).await.ok();
                        }
                    }
                    Err(e) => {
                        tx.send(Event::Error(format!("Generation error: {}", e)))
                            .await
                            .ok();
                        tx.send(Event::GenerationDone).await.ok();
                    }
                }
            }
        });

        self.generation_task = Some(handle.abort_handle());
    }

    fn handle_fetch_sessions(&self) {
        let repo = self.repo.clone();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            match crate::db::repo::get_sessions(&repo.conn).await {
                Ok(sessions) => {
                    tx.send(Event::SessionsFetched(sessions)).await.ok();
                }
                Err(e) => {
                    tx.send(Event::Error(format!("Fetch sessions failed: {}", e)))
                        .await
                        .ok();
                }
            }
        });
    }

    fn handle_load_session(&self, id: SessionId) {
        let repo = self.repo.clone();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            match crate::db::repo::get_messages(&repo.conn, &id).await {
                Ok(messages) => {
                    tx.send(Event::MessagesLoaded(messages)).await.ok();
                }
                Err(e) => {
                    tx.send(Event::Error(format!("Load messages failed: {}", e)))
                        .await
                        .ok();
                }
            }
        });
    }

    fn handle_create_session(&self, id: SessionId, title: String, model: ModelName) {
        let repo = self.repo.clone();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = crate::db::repo::create_session(&repo.conn, &id, &title, &model).await {
                tx.send(Event::Error(format!("Create session failed: {}", e)))
                    .await
                    .ok();
            }
        });
    }

    fn handle_delete_session(&self, id: SessionId) {
        let repo = self.repo.clone();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = crate::db::repo::delete_session(&repo.conn, &id).await {
                tx.send(Event::Error(format!("Delete session failed: {}", e)))
                    .await
                    .ok();
            }
        });
    }

    fn handle_save_message(&self, id: SessionId, role: String, content: String) {
        let repo = self.repo.clone();

        tokio::spawn(async move {
            if let Err(e) = crate::db::repo::add_message(&repo.conn, &id, &role, &content).await {
                tracing::error!("Failed to save message: {}", e);
            }
        });
    }
}
