use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::config::{resolve_secret, Config};

use super::{Message, Role};

#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum AnthropicModel {
    #[serde(alias = "claude-3-5-haiku-20241022")]
    #[serde(rename = "claude-3-5-haiku-latest")]
    Claude35Haiku,
    #[default]
    #[serde(alias = "claude-3-7-sonnet-20250219")]
    #[serde(rename = "claude-3-7-sonnet-latest")]
    Claude37Sonnet,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingType {
    #[default]
    Enabled,
}

#[derive(Debug, Serialize)]
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    pub thinking_type: ThinkingType,
    pub budget_tokens: i32,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        ThinkingConfig {
            thinking_type: ThinkingType::default(),
            budget_tokens: 1024,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct AnthropicRequest {
    pub model: AnthropicModel,
    pub messages: Vec<Message>,
    pub max_tokens: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    // This is for type=thinking
    pub thinking: Option<String>,
    // This is for type=redacted-thinking
    pub data: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: AnthropicModel,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

pub struct AnthropicClient {
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn create_chat_completion(
        &self,
        request: AnthropicRequest,
    ) -> anyhow::Result<AnthropicResponse> {
        let response = ureq::post("https://api.anthropic.com/v1/messages")
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", "2023-06-01")
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        Ok(response)
    }
}

impl super::LlmProvider for AnthropicClient {
    fn from_config(config: Config) -> anyhow::Result<Self> {
        let api_key_raw = config
            .anthropic_api_key
            .ok_or(anyhow!("Missing anthropic_api_key in config"))?;
        let api_key = resolve_secret(&api_key_raw)?;
        Ok(Self::new(api_key))
    }
}

impl super::Chat for AnthropicClient {
    fn create_message(&self, request: super::ChatRequest) -> anyhow::Result<super::ChatResponse> {
        let result = self.create_chat_completion(AnthropicRequest {
            messages: vec![super::Message {
                role: super::Role::User,
                content: request.query,
            }],
            model: match request.model_type {
                super::ModelType::Small => AnthropicModel::Claude35Haiku,
                super::ModelType::Large => AnthropicModel::Claude37Sonnet,
                super::ModelType::Thinking => AnthropicModel::Claude37Sonnet,
            },
            // Double max tokens for thinking to account for thinking tokens
            max_tokens: match request.model_type {
                super::ModelType::Thinking => 2048,
                _ => 1024,
            },
            thinking: match request.model_type {
                super::ModelType::Thinking => Some(ThinkingConfig::default()),
                _ => None,
            },
            ..AnthropicRequest::default()
        })?;
        let content = result
            .content
            .into_iter()
            // Filter out thinking blocks
            .filter(|message| message.content_type == "text")
            .nth(0)
            .expect("It should always return some content");
        Ok(super::ChatResponse {
            message: Message {
                content: content
                    .text
                    .expect("text type content should have text field"),
                role: result.role,
            },
        })
    }
}
