use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd)]
pub struct Model {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: Option<ModelDetails>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd)]
pub struct ModelDetails {
    pub format: String,
    pub family: String,
    pub families: Option<Vec<String>>,
    pub parameter_size: String,
    pub quantization_level: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListModelsResponse {
    pub models: Vec<Model>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ShowModelRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ShowModelResponse {
    pub license: Option<String>,
    pub modelfile: Option<String>,
    pub parameters: Option<String>,
    pub template: Option<String>,
    pub details: Option<ModelDetails>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i32>>, // For history (legacy), usually handled by maintaining conversation
    pub stream: Option<bool>,
    // Add other options as needed
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GenerateResponse {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
    pub context: Option<Vec<i32>>,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u32>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u32>,
    pub eval_duration: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullResponse {
    pub status: String,
    pub digest: Option<String>,
    pub total: Option<u64>,
    pub completed: Option<u64>,
}