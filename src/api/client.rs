use crate::api::types::*;
use anyhow::Result;
use async_stream::try_stream;
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

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

    pub async fn list_models(&self) -> Result<ListModelsResponse> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        let models = resp.json().await?;
        Ok(models)
    }

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
            let bytes_stream = res.bytes_stream().map(|res| {
                res.map_err(std::io::Error::other)
            });
            let reader = StreamReader::new(bytes_stream);
            let mut framed = FramedRead::new(reader, LinesCodec::new());

            while let Some(line) = framed.next().await {
                let line = line?;
                let line = line.trim();
                if !line.is_empty() {
                    let resp: GenerateResponse = serde_json::from_str(line)?;
                    yield resp;
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
            let bytes_stream = res.bytes_stream().map(|res| {
                res.map_err(std::io::Error::other)
            });
            let reader = StreamReader::new(bytes_stream);
            let mut framed = FramedRead::new(reader, LinesCodec::new());

            while let Some(line) = framed.next().await {
                let line = line?;
                let line = line.trim();
                if !line.is_empty() {
                    let resp: PullResponse = serde_json::from_str(line)?;
                    yield resp;
                }
            }
        }
    }
}
