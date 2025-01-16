use serde::{Deserialize, Serialize};
use ureq::Agent;

use super::Message;

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum PerplexityModel {
    #[serde(rename = "llama-3.1-sonar-small-128k-online")]
    Llama31SonarSmall,
    #[default]
    #[serde(rename = "llama-3.1-sonar-large-128k-online")]
    Llama31SonarLarge,
    #[serde(rename = "llama-3.1-sonar-huge-128k-online")]
    Llama31SonarHuge,
}


#[derive(Debug, Default, Serialize)]
pub struct PerplexityRequest {
    pub messages: Vec<Message>,
    pub model: PerplexityModel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
    pub finish_reason: Option<String>,
    pub index: i32,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub completion_tokens: i32,
    pub prompt_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Deserialize)]
pub struct PerplexityResponse {
    pub choices: Vec<Choice>,
    pub created: i64,
    pub id: String,
    pub model: PerplexityModel,
    pub usage: Usage,
    pub citations: Option<Vec<String>>,
}

pub struct PerplexityClient {
    agent: Agent,
    api_key: String,
}

impl PerplexityClient {
    pub fn new(api_key: String) -> Self {
        Self {
            agent: Agent::new(),
            api_key,
        }
    }

    pub fn create_chat_completion(
        &self,
        request: PerplexityRequest,
    ) -> anyhow::Result<PerplexityResponse> {
        let response = self.agent
            .post("https://api.perplexity.ai/chat/completions")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(&request)?;

        let completion = response.into_json::<PerplexityResponse>()?;
        Ok(completion)
    }
}
