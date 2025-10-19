use std::{
    env,
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
    anthropic::AnthropicClient, ollama::OllamaClient, perplexity::PerplexityClient,
    vertex_ai::VertexAiClient, Chat, GroundedChat, LlmProvider,
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
    Ollama,
}

impl ChatProvider {
    pub fn create_client(&self, config: Config) -> anyhow::Result<Box<dyn Chat>> {
        match self {
            ChatProvider::VertexAI => Ok(Box::new(VertexAiClient::from_config(config)?)),
            ChatProvider::Anthropic => Ok(Box::new(AnthropicClient::from_config(config)?)),
            ChatProvider::Ollama => Ok(Box::new(OllamaClient::from_config(config)?)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct VertexAiConfig {
    pub api_key: String,
    pub project_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct JiraConfig {
    pub instance_url: String,
    pub email: String,
    pub api_token: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
/// Configuration for the Ollama chat provider.
pub struct OllamaConfig {
    /// Base URL for the Ollama server. Defaults to `http://localhost:11434` when omitted or left
    /// blank.
    #[serde(default = "default_ollama_base_url")]
    pub base_url: String,
    /// Model to use for [`ModelType::Small`](crate::llm::ModelType).
    pub small: Option<String>,
    /// Model to use for [`ModelType::Large`](crate::llm::ModelType). Falls back to the `small`
    /// model when unspecified.
    pub large: Option<String>,
    /// Model to use for [`ModelType::Thinking`](crate::llm::ModelType). Falls back to the `large`
    /// model when unspecified.
    pub thinking: Option<String>,
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
    pub ollama: Option<OllamaConfig>,
    /// GitHub API tokens mapped by host (e.g., github.com or github.example.com)
    #[serde(default)]
    pub github_tokens: HashMap<String, String>,
    /// Jira configuration
    pub jira: Option<JiraConfig>,
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
    #[serde(default)]
    pub test_commands: Vec<String>,
    #[serde(default)]
    pub fmt_commands: Vec<String>,
    #[serde(default)]
    pub build_commands: Vec<String>,
}

impl RepoConfig {
    pub fn merge_with(mut self, other: RepoConfig) -> Self {
        if !other.pre_start_commands.is_empty() {
            self.pre_start_commands = other.pre_start_commands;
        }
        if !other.post_start_commands.is_empty() {
            self.post_start_commands = other.post_start_commands;
        }
        if !other.pre_end_commands.is_empty() {
            self.pre_end_commands = other.pre_end_commands;
        }
        if !other.post_end_commands.is_empty() {
            self.post_end_commands = other.post_end_commands;
        }
        if !other.test_commands.is_empty() {
            self.test_commands = other.test_commands;
        }
        if !other.fmt_commands.is_empty() {
            self.fmt_commands = other.fmt_commands;
        }
        if !other.build_commands.is_empty() {
            self.build_commands = other.build_commands;
        }
        self
    }
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

        if self.ollama.is_some() {
            return Some(ChatProvider::Ollama);
        }

        None
    }
}

fn default_repo_base_dir() -> String {
    "~/repos/".to_string()
}

pub(crate) fn default_ollama_base_url() -> String {
    "http://localhost:11434".to_string()
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
            .with_context(|| format!("Failed to run command: {cmd}"))?;
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
        std::env::var(env_var).with_context(|| format!("{env_var} env var doesn't exist"))
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
    // Start with default config
    let mut config: RepoConfig = toml::from_str("")?;

    // Load .git/info/wkfl.toml if it exists
    let git_config_file = repo_root_dir.join(".git/info/wkfl.toml");
    if git_config_file.exists() {
        let git_config_str = read_to_string(git_config_file)?;
        let git_config: RepoConfig = toml::from_str(&git_config_str)?;
        config = config.merge_with(git_config);
    }

    // Load .wkfl.toml from current directory if it exists (overrides git config)
    let current_dir = env::current_dir()?;
    let current_config_file = current_dir.join(".wkfl.toml");
    if current_config_file.exists() {
        let current_config_str = read_to_string(current_config_file)?;
        let current_config: RepoConfig = toml::from_str(&current_config_str)?;
        config = config.merge_with(current_config);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_with_empty_configs() {
        let base = RepoConfig {
            pre_start_commands: vec![],
            post_start_commands: vec![],
            pre_end_commands: vec![],
            post_end_commands: vec![],
            test_commands: vec![],
            fmt_commands: vec![],
            build_commands: vec![],
        };
        let other = RepoConfig {
            pre_start_commands: vec![],
            post_start_commands: vec![],
            pre_end_commands: vec![],
            post_end_commands: vec![],
            test_commands: vec![],
            fmt_commands: vec![],
            build_commands: vec![],
        };

        let result = base.merge_with(other);

        assert!(result.pre_start_commands.is_empty());
        assert!(result.post_start_commands.is_empty());
        assert!(result.pre_end_commands.is_empty());
        assert!(result.post_end_commands.is_empty());
        assert!(result.test_commands.is_empty());
        assert!(result.fmt_commands.is_empty());
        assert!(result.build_commands.is_empty());
    }

    #[test]
    fn test_merge_with_base_has_values_other_empty() {
        let base = RepoConfig {
            pre_start_commands: vec!["base_pre_start".to_string()],
            post_start_commands: vec!["base_post_start".to_string()],
            pre_end_commands: vec!["base_pre_end".to_string()],
            post_end_commands: vec!["base_post_end".to_string()],
            test_commands: vec!["base_test".to_string()],
            fmt_commands: vec!["base_fmt".to_string()],
            build_commands: vec!["base_build".to_string()],
        };
        let other = RepoConfig {
            pre_start_commands: vec![],
            post_start_commands: vec![],
            pre_end_commands: vec![],
            post_end_commands: vec![],
            test_commands: vec![],
            fmt_commands: vec![],
            build_commands: vec![],
        };

        let result = base.merge_with(other);

        assert_eq!(result.pre_start_commands, vec!["base_pre_start"]);
        assert_eq!(result.post_start_commands, vec!["base_post_start"]);
        assert_eq!(result.pre_end_commands, vec!["base_pre_end"]);
        assert_eq!(result.post_end_commands, vec!["base_post_end"]);
        assert_eq!(result.test_commands, vec!["base_test"]);
        assert_eq!(result.fmt_commands, vec!["base_fmt"]);
        assert_eq!(result.build_commands, vec!["base_build"]);
    }

    #[test]
    fn test_merge_with_base_empty_other_has_values() {
        let base = RepoConfig {
            pre_start_commands: vec![],
            post_start_commands: vec![],
            pre_end_commands: vec![],
            post_end_commands: vec![],
            test_commands: vec![],
            fmt_commands: vec![],
            build_commands: vec![],
        };
        let other = RepoConfig {
            pre_start_commands: vec!["other_pre_start".to_string()],
            post_start_commands: vec!["other_post_start".to_string()],
            pre_end_commands: vec!["other_pre_end".to_string()],
            post_end_commands: vec!["other_post_end".to_string()],
            test_commands: vec!["other_test".to_string()],
            fmt_commands: vec!["other_fmt".to_string()],
            build_commands: vec!["other_build".to_string()],
        };

        let result = base.merge_with(other);

        assert_eq!(result.pre_start_commands, vec!["other_pre_start"]);
        assert_eq!(result.post_start_commands, vec!["other_post_start"]);
        assert_eq!(result.pre_end_commands, vec!["other_pre_end"]);
        assert_eq!(result.post_end_commands, vec!["other_post_end"]);
        assert_eq!(result.test_commands, vec!["other_test"]);
        assert_eq!(result.fmt_commands, vec!["other_fmt"]);
        assert_eq!(result.build_commands, vec!["other_build"]);
    }

    #[test]
    fn test_merge_with_both_have_values_other_overrides() {
        let base = RepoConfig {
            pre_start_commands: vec!["base_pre_start".to_string()],
            post_start_commands: vec!["base_post_start".to_string()],
            pre_end_commands: vec!["base_pre_end".to_string()],
            post_end_commands: vec!["base_post_end".to_string()],
            test_commands: vec!["base_test".to_string()],
            fmt_commands: vec!["base_fmt".to_string()],
            build_commands: vec!["base_build".to_string()],
        };
        let other = RepoConfig {
            pre_start_commands: vec!["other_pre_start".to_string()],
            post_start_commands: vec!["other_post_start".to_string()],
            pre_end_commands: vec!["other_pre_end".to_string()],
            post_end_commands: vec!["other_post_end".to_string()],
            test_commands: vec!["other_test".to_string()],
            fmt_commands: vec!["other_fmt".to_string()],
            build_commands: vec!["other_build".to_string()],
        };

        let result = base.merge_with(other);

        assert_eq!(result.pre_start_commands, vec!["other_pre_start"]);
        assert_eq!(result.post_start_commands, vec!["other_post_start"]);
        assert_eq!(result.pre_end_commands, vec!["other_pre_end"]);
        assert_eq!(result.post_end_commands, vec!["other_post_end"]);
        assert_eq!(result.test_commands, vec!["other_test"]);
        assert_eq!(result.fmt_commands, vec!["other_fmt"]);
        assert_eq!(result.build_commands, vec!["other_build"]);
    }

    #[test]
    fn test_merge_with_partial_override() {
        let base = RepoConfig {
            pre_start_commands: vec!["base_pre_start".to_string()],
            post_start_commands: vec!["base_post_start".to_string()],
            pre_end_commands: vec!["base_pre_end".to_string()],
            post_end_commands: vec!["base_post_end".to_string()],
            test_commands: vec!["base_test".to_string()],
            fmt_commands: vec!["base_fmt".to_string()],
            build_commands: vec!["base_build".to_string()],
        };
        let other = RepoConfig {
            pre_start_commands: vec![], // Empty, should not override
            post_start_commands: vec!["other_post_start".to_string()], // Should override
            pre_end_commands: vec![],   // Empty, should not override
            post_end_commands: vec![],  // Empty, should not override
            test_commands: vec!["other_test1".to_string(), "other_test2".to_string()], // Should override
            fmt_commands: vec![], // Empty, should not override
            build_commands: vec!["other_build1".to_string(), "other_build2".to_string()], // Should override
        };

        let result = base.merge_with(other);

        assert_eq!(result.pre_start_commands, vec!["base_pre_start"]); // Kept from base
        assert_eq!(result.post_start_commands, vec!["other_post_start"]); // Overridden
        assert_eq!(result.pre_end_commands, vec!["base_pre_end"]); // Kept from base
        assert_eq!(result.post_end_commands, vec!["base_post_end"]); // Kept from base
        assert_eq!(result.test_commands, vec!["other_test1", "other_test2"]); // Overridden
        assert_eq!(result.fmt_commands, vec!["base_fmt"]); // Kept from base
        assert_eq!(result.build_commands, vec!["other_build1", "other_build2"]);
        // Overridden
    }

    #[test]
    fn test_merge_with_multiple_commands() {
        let base = RepoConfig {
            pre_start_commands: vec!["base1".to_string(), "base2".to_string()],
            post_start_commands: vec![],
            pre_end_commands: vec![],
            post_end_commands: vec![],
            test_commands: vec![],
            fmt_commands: vec![],
            build_commands: vec![],
        };
        let other = RepoConfig {
            pre_start_commands: vec![], // Should not override
            post_start_commands: vec![
                "other1".to_string(),
                "other2".to_string(),
                "other3".to_string(),
            ],
            pre_end_commands: vec![],
            post_end_commands: vec![],
            test_commands: vec!["test1".to_string()],
            fmt_commands: vec![],
            build_commands: vec!["build1".to_string()],
        };

        let result = base.merge_with(other);

        assert_eq!(result.pre_start_commands, vec!["base1", "base2"]); // Kept from base
        assert_eq!(
            result.post_start_commands,
            vec!["other1", "other2", "other3"]
        ); // From other
        assert!(result.pre_end_commands.is_empty());
        assert!(result.post_end_commands.is_empty());
        assert_eq!(result.test_commands, vec!["test1"]); // From other
        assert!(result.fmt_commands.is_empty()); // Empty from both
        assert_eq!(result.build_commands, vec!["build1"]); // From other
    }

    #[test]
    fn test_merge_with_chaining() {
        let base = RepoConfig {
            pre_start_commands: vec!["base".to_string()],
            post_start_commands: vec![],
            pre_end_commands: vec![],
            post_end_commands: vec![],
            test_commands: vec![],
            fmt_commands: vec![],
            build_commands: vec![],
        };
        let middle = RepoConfig {
            pre_start_commands: vec![], // Should not override
            post_start_commands: vec!["middle".to_string()],
            pre_end_commands: vec![],
            post_end_commands: vec![],
            test_commands: vec![],
            fmt_commands: vec!["middle_fmt".to_string()],
            build_commands: vec!["middle_build".to_string()],
        };
        let final_config = RepoConfig {
            pre_start_commands: vec!["final".to_string()], // Should override
            post_start_commands: vec![],                   // Should not override
            pre_end_commands: vec![],
            post_end_commands: vec![],
            test_commands: vec!["final_test".to_string()],
            fmt_commands: vec![],   // Should not override
            build_commands: vec![], // Should not override
        };

        let result = base.merge_with(middle).merge_with(final_config);

        assert_eq!(result.pre_start_commands, vec!["final"]); // From final
        assert_eq!(result.post_start_commands, vec!["middle"]); // From middle
        assert!(result.pre_end_commands.is_empty());
        assert!(result.post_end_commands.is_empty());
        assert_eq!(result.test_commands, vec!["final_test"]); // From final
        assert_eq!(result.fmt_commands, vec!["middle_fmt"]); // From middle (final is empty)
        assert_eq!(result.build_commands, vec!["middle_build"]); // From middle (final is empty)
    }
}
