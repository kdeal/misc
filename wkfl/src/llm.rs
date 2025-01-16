use serde::{Deserialize, Serialize};

pub mod anthropic;
pub mod perplexity;

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
