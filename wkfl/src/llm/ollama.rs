use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

use crate::{
    config::{default_ollama_base_url, Config, OllamaConfig},
    llm::{ChatRequest, ChatResponse, LlmProvider, Message, ModelType, Role},
};

const CHAT_ENDPOINT: &str = "/api/chat";

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<Message>,
}

pub struct OllamaClient {
    base_url: String,
    small_model: String,
    large_model: String,
    thinking_model: String,
}

impl OllamaClient {
    fn model_for_type(&self, model_type: ModelType) -> &str {
        match model_type {
            ModelType::Small => &self.small_model,
            ModelType::Large => &self.large_model,
            ModelType::Thinking => &self.thinking_model,
        }
    }

    fn send_chat_request(&self, request: &OllamaChatRequest) -> anyhow::Result<OllamaChatResponse> {
        let response = ureq::post(&format!("{}{}", self.base_url, CHAT_ENDPOINT))
            .set("Content-Type", "application/json")
            .send_json(request)
            .with_context(|| "Failed to call Ollama chat endpoint".to_string())?;

        response
            .into_json::<OllamaChatResponse>()
            .with_context(|| "Failed to parse Ollama response".to_string())
    }
}

fn sanitize_base_url(config: &OllamaConfig) -> String {
    let trimmed = config.base_url.trim();
    let base = if trimmed.is_empty() {
        default_ollama_base_url()
    } else {
        trimmed.to_string()
    };

    base.trim_end_matches('/').to_string()
}

impl LlmProvider for OllamaClient {
    fn from_config(config: Config) -> anyhow::Result<Self> {
        let ollama_config = config
            .ollama
            .ok_or(anyhow!("Missing [ollama] configuration"))?;

        let base_url = sanitize_base_url(&ollama_config);
        let small_model = ollama_config.small.ok_or(anyhow!(
            "Missing small model name in [ollama] configuration"
        ))?;
        let large_model = ollama_config.large.unwrap_or_else(|| small_model.clone());
        let thinking_model = ollama_config
            .thinking
            .unwrap_or_else(|| large_model.clone());

        Ok(Self {
            base_url,
            small_model,
            large_model,
            thinking_model,
        })
    }
}

impl crate::llm::Chat for OllamaClient {
    fn create_message(&self, request: ChatRequest) -> anyhow::Result<ChatResponse> {
        let ChatRequest { query, model_type } = request;
        let ollama_request = OllamaChatRequest {
            model: self.model_for_type(model_type).to_string(),
            messages: vec![Message {
                role: Role::User,
                content: query,
            }],
            stream: false,
        };

        let response = self.send_chat_request(&ollama_request)?;
        let response_message = response
            .message
            .ok_or_else(|| anyhow!("Ollama response did not contain a message"))?;

        Ok(ChatResponse {
            message: response_message,
        })
    }
}
