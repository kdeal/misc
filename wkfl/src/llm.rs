use std::io::{self, IsTerminal, Read};

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::{config::Config, prompts::basic_prompt};

pub mod anthropic;
pub mod perplexity;
pub mod vertex_ai;

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub query: String,
    pub model_type: ModelType,
}

#[derive(Debug, Serialize)]
pub struct GroundedChatRequest {
    pub query: String,
    pub model_type: ModelType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Assistant,
    System,
    User,
}

#[derive(Clone, Debug, Default, Serialize, ValueEnum)]
pub enum ModelType {
    #[default]
    Small,
    Large,
    Thinking,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug)]
pub struct ChatResponse {
    pub message: Message,
}

#[derive(Debug)]
pub struct GroundedChatResponse {
    pub message: Message,
    pub citations: CitationMetadata,
}

#[derive(Debug)]
pub struct CitationMetadata {
    pub sources: Vec<Source>,
    pub supports: Vec<Support>,
}

#[derive(Debug, Deserialize)]
pub struct Source {
    pub title: String,
    pub uri: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Support {
    pub start_index: usize,
    pub end_index: usize,
    pub text: String,
    pub source_indices: Vec<u8>,
}

pub trait LlmProvider: Sized {
    fn from_config(config: Config) -> anyhow::Result<Self>;
}

pub trait GroundedChat {
    fn create_grounded_chat_completion(
        &self,
        request: GroundedChatRequest,
    ) -> anyhow::Result<GroundedChatResponse>;
}

pub trait Chat {
    fn create_message(&self, request: ChatRequest) -> anyhow::Result<ChatResponse>;
}

pub fn get_query(maybe_query: Option<String>) -> Result<String> {
    if let Some(query) = maybe_query {
        return Ok(query);
    }

    let mut stdin = io::stdin();
    if stdin.is_terminal() {
        return basic_prompt("Query:");
    }
    let mut query = String::new();
    stdin.read_to_string(&mut query)?;
    Ok(query)
}
