use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use home::home_dir;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    #[serde(default = "default_repo_base_dir")]
    repositories_directory: String,
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
        if self.repositories_directory.starts_with("~/") {
            let mut repo_path = home_dir().ok_or(anyhow::anyhow!("Can't determine home dir"))?;
            let no_prefix_repos_dir = self
                .repositories_directory
                .strip_prefix("~/")
                .expect("Checked that it had the prefix above. Should be safe");
            repo_path.push(no_prefix_repos_dir);
            Ok(repo_path)
        } else {
            Ok(PathBuf::from(&self.repositories_directory))
        }
    }
}

fn default_repo_base_dir() -> String {
    "~/repos/".to_string()
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
