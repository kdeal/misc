use std::io::{self, BufRead, BufReader, IsTerminal, Read};

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

#[derive(Debug, Deserialize, Clone)]
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

#[allow(dead_code)]
#[derive(Debug)]
pub struct ServerSentEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

pub struct SseReader {
    reader: BufReader<Box<dyn Read>>,
    done: bool,
}

impl SseReader {
    pub fn new(reader: Box<dyn Read>) -> Self {
        Self {
            reader: BufReader::new(reader),
            done: false,
        }
    }
}

impl Iterator for SseReader {
    type Item = anyhow::Result<ServerSentEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let mut event: Option<String> = None;
        let mut data = Vec::<String>::new();
        let mut id: Option<String> = None;
        let mut retry: Option<u64> = None;

        loop {
            let mut line_buf = String::new();
            let bytes_read_res = self.reader.read_line(&mut line_buf);
            let bytes_read = match bytes_read_res {
                Ok(val) => val,
                Err(e) => return Some(Err(anyhow::Error::from(e))),
            };

            if bytes_read == 0 {
                self.done = true;
            }

            let line = line_buf.trim();

            // Empty line marks the end of an event
            if line.is_empty() || bytes_read == 0 {
                // If we don't have data keep going
                if data.is_empty() {
                    if self.done {
                        return None;
                    } else {
                        continue;
                    }
                }

                let data_str = data.join("\n");

                return Some(Ok(ServerSentEvent {
                    event,
                    data: data_str,
                    id,
                    retry,
                }));
            }

            // Ignore comment lines
            if line.starts_with(':') {
                continue;
            }

            // Parse field
            if let Some((field, value)) = line.split_once(':') {
                let value = value.strip_prefix(' ').unwrap_or(value);

                match field {
                    "event" => event = Some(value.to_string()),
                    "data" => data.push(value.to_string()),
                    "id" => id = Some(value.to_string()),
                    "retry" => {
                        if let Ok(ms) = value.parse::<u64>() {
                            retry = Some(ms);
                        }
                    }
                    _ => {} // Ignore other fields
                }
            } else {
                // Line without colon is treated as field name with empty value
                match line {
                    "event" => event = Some(String::new()),
                    "data" => data.push(String::new()),
                    "id" => id = Some(String::new()),
                    "retry" => retry = None,
                    _ => {} // Ignore other fields
                }
            }
        }
    }
}
