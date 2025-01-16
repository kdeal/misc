use serde::{Deserialize, Serialize};

use super::{Message, Role};

#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum AnthropicModel {
    #[serde(alias = "claude-3-5-haiku-20241022")]
    #[serde(rename = "claude-3-5-haiku-latest")]
    Claude35Haiku,
    #[default]
    #[serde(alias = "claude-3-5-sonnet-20241022")]
    #[serde(rename = "claude-3-5-sonnet-latest")]
    Claude35Sonnet,
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
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
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
