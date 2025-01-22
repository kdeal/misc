use std::io::{self, IsTerminal, Read};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::prompts::basic_prompt;

pub mod anthropic;
pub mod perplexity;
pub mod vertex_ai;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Assistant,
    System,
    User,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
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
