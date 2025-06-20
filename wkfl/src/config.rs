use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Ok};
use clap::ValueEnum;
use home::home_dir;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::llm::{
    anthropic::AnthropicClient, perplexity::PerplexityClient, vertex_ai::VertexAiClient, Chat,
    GroundedChat, LlmProvider,
};

#[derive(Serialize, Deserialize, Clone, Debug, ValueEnum)]
pub enum WebChatProvider {
    VertexAI,
    Perplexity,
}

impl WebChatProvider {
    pub fn create_client(&self, config: Config) -> anyhow::Result<Box<dyn GroundedChat>> {
        match self {
            WebChatProvider::VertexAI => Ok(Box::new(VertexAiClient::from_config(config)?)),
            WebChatProvider::Perplexity => Ok(Box::new(PerplexityClient::from_config(config)?)),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, ValueEnum)]
pub enum ChatProvider {
    VertexAI,
    Anthropic,
}

impl ChatProvider {
    pub fn create_client(&self, config: Config) -> anyhow::Result<Box<dyn Chat>> {
        match self {
            ChatProvider::VertexAI => Ok(Box::new(VertexAiClient::from_config(config)?)),
            ChatProvider::Anthropic => Ok(Box::new(AnthropicClient::from_config(config)?)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct VertexAiConfig {
    pub api_key: String,
    pub project_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    #[serde(default = "default_repo_base_dir")]
    repositories_directory: String,
    notes_directory: Option<String>,
    web_chat_provider: Option<WebChatProvider>,
    chat_provider: Option<ChatProvider>,

    pub anthropic_api_key: Option<String>,
    pub perplexity_api_key: Option<String>,
    pub vertex_ai: Option<VertexAiConfig>,
    /// GitHub API tokens mapped by host (e.g., github.com or github.example.com)
    #[serde(default)]
    pub github_tokens: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RepoConfig {
    #[serde(default)]
    pub pre_start_commands: Vec<String>,
    #[serde(default)]
    pub post_start_commands: Vec<String>,
    #[serde(default)]
    pub pre_end_commands: Vec<String>,
    #[serde(default)]
    pub post_end_commands: Vec<String>,
}

impl Config {
    pub fn repositories_directory_path(&self) -> anyhow::Result<PathBuf> {
        create_path_from_string(&self.repositories_directory)
    }
    pub fn notes_directory_path(&self) -> anyhow::Result<PathBuf> {
        if let Some(notes_directory) = &self.notes_directory {
            create_path_from_string(notes_directory)
        } else {
            let mut notes_directory_path = self.repositories_directory_path()?;
            notes_directory_path.push("notes");
            Ok(notes_directory_path)
        }
    }

    pub fn get_web_chat_provider(&self) -> Option<WebChatProvider> {
        if self.web_chat_provider.is_some() {
            return self.web_chat_provider.clone();
        }

        if self.perplexity_api_key.is_some() {
            return Some(WebChatProvider::Perplexity);
        }

        if self.vertex_ai.is_some() {
            return Some(WebChatProvider::VertexAI);
        }

        None
    }
    pub fn get_chat_provider(&self) -> Option<ChatProvider> {
        if self.chat_provider.is_some() {
            return self.chat_provider.clone();
        }

        if self.anthropic_api_key.is_some() {
            return Some(ChatProvider::Anthropic);
        }

        if self.vertex_ai.is_some() {
            return Some(ChatProvider::VertexAI);
        }

        None
    }
}

fn default_repo_base_dir() -> String {
    "~/repos/".to_string()
}

/// Creates a PathBuf from a string. Handles converting ~/ to home dir
fn create_path_from_string(path_str: &str) -> anyhow::Result<PathBuf> {
    if path_str.starts_with("~/") {
        let mut path = home_dir().ok_or(anyhow::anyhow!("Can't determine home dir"))?;
        let no_prefix_path = path_str
            .strip_prefix("~/")
            .expect("Checked that it had the prefix above. Should be safe");
        path.push(no_prefix_path);
        Ok(path)
    } else {
        Ok(PathBuf::from(path_str))
    }
}

pub fn resolve_secret(config_value: &str) -> anyhow::Result<String> {
    if config_value.starts_with("cmd::") {
        let cmd = config_value
            .strip_prefix("cmd::")
            .expect("We check the prefix above, so this shouldn't fail");
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .with_context(|| format!("Failed to run command: {}", cmd))?;
        if !output.status.success() {
            bail!("Command failed: {}", cmd);
        }
        let cmd_output = String::from_utf8(output.stdout)
            .with_context(|| "Failed to parse result of cmd as utf".to_string())?;
        Ok(cmd_output.trim().to_string())
    } else if config_value.starts_with("env::") {
        let env_var = config_value
            .strip_prefix("env::")
            .expect("We check the prefix above, so this shouldn't fail");
        std::env::var(env_var).with_context(|| format!("{} env var doesn't exist", env_var))
    } else if config_value.starts_with("val::") {
        let value = config_value
            .strip_prefix("val::")
            .expect("We check the prefix above, so this shouldn't fail");
        Ok(value.to_string())
    } else {
        Ok(config_value.to_string())
    }
}

pub fn get_config() -> anyhow::Result<Config> {
    let mut config_buf = home_dir().ok_or(anyhow::anyhow!("Can't determine home dir"))?;

    config_buf.push(".config/wkfl/");
    let config_dir = config_buf.as_path();
    if !config_dir.exists() {
        return Ok(toml::from_str("")?);
    }

    config_buf.push("config.toml");
    let config_file = config_buf.as_path();
    if !config_file.exists() {
        return Ok(toml::from_str("")?);
    }

    let config_str = read_to_string(config_file)?;
    let config = toml::from_str(&config_str)?;
    Ok(config)
}

pub fn get_repo_config(repo_root_dir: &Path) -> anyhow::Result<RepoConfig> {
    let config_file = repo_root_dir.join(".git/info/wkfl.toml");
    if !config_file.exists() {
        return Ok(toml::from_str("")?);
    }

    let config_str = read_to_string(config_file)?;
    let config = toml::from_str(&config_str)?;
    Ok(config)
}
