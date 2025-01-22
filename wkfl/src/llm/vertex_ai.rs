use serde::{Deserialize, Serialize};
use serde_json;
use std::fmt;

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum VertexAiModel {
    #[default]
    #[serde(rename = "gemini-2.0-flash-exp")]
    Gemini20Flash,
    #[serde(rename = "gemini-1.5-flash-002")]
    Gemini15Flash,
    #[serde(rename = "gemini-1.5-pro-002")]
    Gemini15Pro,
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
    pub generation_config: Option<GenerationConfig>,
}

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
    pub citation_metadata: Option<CitationMetadata>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationMetadata {
    pub citations: Vec<Citation>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub start_index: i32,
    pub end_index: i32,
    pub uri: String,
    pub title: String,
    pub license: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication_date: Option<PublicationDate>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationDate {
    pub year: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: i32,
    pub total_token_count: i32,
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

    pub fn create_chat_completion(
        &self,
        request: VertexAiRequest,
        model: VertexAiModel,
    ) -> anyhow::Result<VertexAiResponse> {
        let url = format!("https://us-central1-aiplatform.googleapis.com/v1/projects/{}/locations/us-central1/publishers/google/models/{}:generateContent", self.project_id, model);
        let response = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(&request)?;

        let completion = response.into_json::<VertexAiResponse>()?;
        Ok(completion)
    }
}
