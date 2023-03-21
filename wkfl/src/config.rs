use std::fs::{create_dir_all, File, read_to_string};

use home::home_dir;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_repo_base_dir")]
    repositories_directory: String,
}

fn default_repo_base_dir() -> String {
    "~/repos/".to_string()
}

pub fn get_config() -> anyhow::Result<Config> {
    let mut config_buf = home_dir().ok_or(anyhow::anyhow!("Can't determine home dir"))?;
    config_buf.push(".config/wkfl/");
    let config_dir = config_buf.as_path();
    if !config_dir.exists() {
        create_dir_all(config_dir)?;
    }
    config_buf.push("config.toml");
    let config_file = config_buf.as_path();
    if !config_file.exists() {
        File::create(config_file)?;
    }
    let config_str = read_to_string(config_file)?;
    let config = toml::from_str(&config_str)?;
    Ok(config)
}
