use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fmt;

use crate::{
    config::{resolve_secret, Config},
    llm::SseReader,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum VertexAiModel {
    #[default]
    #[serde(rename = "gemini-2.5-flash-preview-04-17")]
    Gemini25Flash,
    #[serde(rename = "gemini-2.5-pro-preview-03-25")]
    Gemini25Pro,
}

impl fmt::Display for VertexAiModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // This is jank, but didn't require a new crate and respects
        // the renames that I need for Deserialize. I already indirectly
        // require serde_json
        let json_repr = serde_json::to_string(self).unwrap();
        write!(f, "{}", json_repr.trim_matches('"'))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VertexAiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GoogleSearchTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSearchTool {
    google_search: GoogleSearchOptions,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSearchOptions {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Model,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_logprobs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_timestamp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VertexAiResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: UsageMetadata,
    pub model_version: VertexAiModel,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_metadata: Option<GroundingMetadata>,
    pub finish_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingMetadata {
    pub grounding_chunks: Vec<GroundingChunk>,
    pub grounding_supports: Vec<GroundingSupport>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroundingChunk {
    pub web: super::Source,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingSupport {
    pub segment: GroundingSupportSegment,
    pub grounding_chunk_indices: Vec<u8>,
    pub confidence_scores: Vec<f32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingSupportSegment {
    #[serde(default = "default_grounding_start_index")]
    pub start_index: usize,
    pub end_index: usize,
    pub text: String,
}

fn default_grounding_start_index() -> usize {
    0
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    // When streaming only the final response has these fields
    pub prompt_token_count: Option<i32>,
    pub candidates_token_count: Option<i32>,
    pub total_token_count: Option<i32>,
}

pub struct VertexAiStreamResponseIterator {
    sse_reader: SseReader,
    done: bool,
}

impl VertexAiStreamResponseIterator {
    pub fn new(sse_reader: SseReader) -> Self {
        Self {
            sse_reader,
            done: false,
        }
    }
}

impl Iterator for VertexAiStreamResponseIterator {
    type Item = anyhow::Result<VertexAiResponse>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Get the next SSE event
        let sse_event = match self.sse_reader.next() {
            Some(Ok(event)) => event,
            None => {
                self.done = true;
                return None;
            }
            Some(Err(e)) => return Some(Err(anyhow!(e))),
        };

        // Parse the stream event
        let event_result = serde_json::from_str::<VertexAiResponse>(&sse_event.data);
        let response = match event_result {
            Ok(event) => event,
            Err(e) => return Some(Err(anyhow!("Failed to parse stream event: {}", e))),
        };

        if response.candidates[0].finish_reason.is_some() {
            self.done = true;
        }

        Some(Ok(response))
    }
}

pub struct VertexAiClient {
    api_key: String,
    project_id: String,
}

impl VertexAiClient {
    pub fn new(api_key: String, project_id: String) -> Self {
        Self {
            api_key,
            project_id,
        }
    }

    fn generate_url(&self, model: VertexAiModel, method: &str) -> String {
        format!(
            "https://us-central1-aiplatform.googleapis.com/v1/projects/{}/locations/us-central1/publishers/google/models/{}:{}",
            self.project_id,
            model,
            method
        )
    }

    pub fn create_chat_completion(
        &self,
        request: VertexAiRequest,
        model: VertexAiModel,
    ) -> anyhow::Result<VertexAiResponse> {
        let url = self.generate_url(model, "generateContent");
        let response = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(&request)?;
        let completion = response.into_json::<VertexAiResponse>()?;
        Ok(completion)
    }

    pub fn stream_chat_completion(
        &self,
        request: VertexAiRequest,
        model: VertexAiModel,
    ) -> anyhow::Result<VertexAiStreamResponseIterator> {
        let url = self.generate_url(model, "streamGenerateContent?alt=sse");

        let response = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(&request)?;

        Ok(VertexAiStreamResponseIterator::new(SseReader::new(
            Box::new(response.into_reader()),
        )))
    }

    fn convert_to_standard_role(role: Option<Role>) -> super::Role {
        match role {
            Some(Role::User) => super::Role::User,
            Some(Role::Model) => super::Role::Assistant,
            None => super::Role::Assistant,
        }
    }

    fn model_from_model_type(model_type: super::ModelType) -> VertexAiModel {
        match model_type {
            super::ModelType::Small => VertexAiModel::Gemini25Flash,
            super::ModelType::Large => VertexAiModel::Gemini25Pro,
            super::ModelType::Thinking => VertexAiModel::Gemini25Pro,
        }
    }
}

impl super::LlmProvider for VertexAiClient {
    fn from_config(config: Config) -> anyhow::Result<Self> {
        let vertex_ai_config = config
            .vertex_ai
            .ok_or(anyhow!("Missing vertex_ai in config"))?;
        let api_key = resolve_secret(&vertex_ai_config.api_key)?;
        Ok(Self::new(api_key, vertex_ai_config.project_id))
    }
}

impl super::GroundedChat for VertexAiClient {
    fn create_grounded_chat_completion(
        &self,
        request: super::GroundedChatRequest,
    ) -> anyhow::Result<super::GroundedChatResponse> {
        let vertex_request = VertexAiRequest {
            contents: vec![Content {
                role: Some(Role::User),
                parts: vec![Part {
                    text: request.query,
                }],
            }],
            ..VertexAiRequest::default()
        };
        let model = Self::model_from_model_type(request.model_type);
        let response = self.create_chat_completion(vertex_request, model)?;
        let candidate = response
            .candidates
            .into_iter()
            .nth(0)
            .expect("It should always return a canidate");
        let grounding_metadata = candidate
            .grounding_metadata
            .unwrap_or(GroundingMetadata::default());
        let mut supports: Vec<super::Support> = grounding_metadata
            .grounding_supports
            .into_iter()
            .map(|support| super::Support {
                start_index: support.segment.start_index,
                end_index: support.segment.end_index,
                text: support.segment.text,
                source_indices: support.grounding_chunk_indices,
            })
            .collect();
        supports.sort_by_key(|support| support.end_index);
        let content = candidate
            .content
            .parts
            .into_iter()
            .nth(0)
            .expect("There should always be one candidate")
            .text;
        Ok(super::GroundedChatResponse {
            message: super::Message {
                role: Self::convert_to_standard_role(candidate.content.role),
                content,
            },
            citations: super::CitationMetadata {
                sources: grounding_metadata
                    .grounding_chunks
                    .into_iter()
                    .map(|chunk| chunk.web)
                    .collect(),
                supports,
            },
        })
    }
}

impl super::Chat for VertexAiClient {
    fn create_message(&self, request: super::ChatRequest) -> anyhow::Result<super::ChatResponse> {
        let vertex_request = VertexAiRequest {
            contents: vec![Content {
                role: Some(Role::User),
                parts: vec![Part {
                    text: request.query,
                }],
            }],
            ..VertexAiRequest::default()
        };
        let model = Self::model_from_model_type(request.model_type);
        let response = self.create_chat_completion(vertex_request, model)?;
        let candidate = response
            .candidates
            .into_iter()
            .nth(0)
            .expect("It should always return a canidate");
        let content = candidate
            .content
            .parts
            .into_iter()
            .nth(0)
            .expect("There should always be one candidate")
            .text;
        Ok(super::ChatResponse {
            message: super::Message {
                content,
                role: Self::convert_to_standard_role(candidate.content.role),
            },
        })
    }
}
