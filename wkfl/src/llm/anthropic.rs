use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::{
    config::{resolve_secret, Config},
    llm::SseReader,
};

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
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Thinking { thinking: String },
    RedactedThinking { data: String },
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Delta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub text: Option<String>,
    pub thinking: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UsageDelta {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart {
        message: AnthropicResponse,
    },
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: ContentDelta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: Delta,
        usage: Option<UsageDelta>,
    },
    MessageStop,
    Ping,
    Error {
        error: StreamError,
    },
}

#[derive(Debug, Deserialize)]
pub struct StreamError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta { text: String },
    ThinkingDelta { thinking: String },
    RedactedThinkingDelta { data: String },
    SignatureDelta { signature: String },
    InputJsonDelta { partial_json: String },
}

pub struct AnthropicStreamResponseIterator {
    sse_reader: SseReader,
    done: bool,
}

impl AnthropicStreamResponseIterator {
    pub fn new(sse_reader: SseReader) -> Self {
        Self {
            sse_reader,
            done: false,
        }
    }
}

impl Iterator for AnthropicStreamResponseIterator {
    type Item = anyhow::Result<StreamEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Get the next SSE event
        let sse_event = match self.sse_reader.next() {
            Some(Ok(event)) => event,
            None => return None,
            Some(Err(e)) => return Some(Err(anyhow!(e))),
        };

        // Parse the stream event
        let event_result = serde_json::from_str::<StreamEvent>(&sse_event.data);
        match event_result {
            Ok(event) => {
                // Check if this is the final message in the stream
                if matches!(event, StreamEvent::MessageStop) {
                    self.done = true;
                }
                Some(Ok(event))
            }
            Err(e) => Some(Err(anyhow!("Failed to parse stream event: {}", e))),
        }
    }
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

    pub fn stream_chat_completion(
        &self,
        mut request: AnthropicRequest,
    ) -> anyhow::Result<AnthropicStreamResponseIterator> {
        // Ensure streaming is enabled
        request.stream = Some(true);

        let response = ureq::post("https://api.anthropic.com/v1/messages")
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", "2023-06-01")
            .set("Content-Type", "application/json")
            .send_json(&request)?;

        Ok(AnthropicStreamResponseIterator::new(SseReader::new(
            Box::new(response.into_reader()),
        )))
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

        for content in result.content {
            if let ContentBlock::Text { text } = content {
                return Ok(super::ChatResponse {
                    message: Message {
                        content: text,
                        role: result.role,
                    },
                });
            }
        }

        Ok(super::ChatResponse {
            message: Message {
                content: "".to_string(),
                role: result.role,
            },
        })
    }
}
