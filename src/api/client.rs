use crate::api::types::*;
use anyhow::Result;
use async_stream::try_stream;
use futures::stream::{Stream, StreamExt};
use reqwest::Client;

#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    #[allow(dead_code)]
    pub async fn check_health(&self) -> Result<bool> {
        let resp = self.client.get(&self.base_url).send().await?;
        Ok(resp.status().is_success())
    }

    pub async fn list_models(&self) -> Result<ListModelsResponse> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        let models = resp.json().await?;
        Ok(models)
    }

    #[allow(dead_code)]
    pub async fn show_model(&self, name: &str) -> Result<ShowModelResponse> {
        let url = format!("{}/api/show", self.base_url);
        let req = ShowModelRequest {
            name: name.to_string(),
        };
        let resp = self.client.post(&url).json(&req).send().await?;
        let info = resp.json().await?;
        Ok(info)
    }

    pub async fn delete_model(&self, name: &str) -> Result<bool> {
        let url = format!("{}/api/delete", self.base_url);
        let resp = self
            .client
            .delete(&url)
            .json(&serde_json::json!({"name": name}))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    pub fn generate_stream(
        &self,
        req: GenerateRequest,
    ) -> impl Stream<Item = Result<GenerateResponse>> + 'static {
        let client = self.client.clone();
        let url = format!("{}/api/generate", self.base_url);

        try_stream! {
            let res = client.post(&url).json(&req).send().await?;
            let mut stream = res.bytes_stream();
            let mut buffer = String::new();

            while let Some(item) = stream.next().await {
                let bytes = item?;
                let s = String::from_utf8_lossy(&bytes);
                buffer.push_str(&s);

                while let Some(pos) = buffer.find('\n') {
                    let line: String = buffer.drain(..pos+1).collect();
                    let line = line.trim();
                    if !line.is_empty() {
                         let resp: GenerateResponse = serde_json::from_str(line)?;
                         yield resp;
                    }
                }
            }
        }
    }

    pub fn pull_model_stream(
        &self,
        req: PullRequest,
    ) -> impl Stream<Item = Result<PullResponse>> + 'static {
        let client = self.client.clone();
        let url = format!("{}/api/pull", self.base_url);

        try_stream! {
            let res = client.post(&url).json(&req).send().await?;
            let mut stream = res.bytes_stream();
            let mut buffer = String::new();

            while let Some(item) = stream.next().await {
                let bytes = item?;
                let s = String::from_utf8_lossy(&bytes);
                buffer.push_str(&s);

                while let Some(pos) = buffer.find('\n') {
                    let line: String = buffer.drain(..pos+1).collect();
                    let line = line.trim();
                    if !line.is_empty() {
                         let resp: PullResponse = serde_json::from_str(line)?;
                         yield resp;
                    }
                }
            }
        }
    }
}
