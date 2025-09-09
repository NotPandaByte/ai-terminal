use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub active_ruleset: Option<String>,
}

pub fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("dev", "ai-terminal", "ai-terminal")
        .ok_or_else(|| anyhow!("unable to resolve project dirs"))
}

pub fn config_dir() -> Result<PathBuf> { Ok(project_dirs()?.config_dir().to_path_buf()) }
pub fn data_dir() -> Result<PathBuf> { Ok(project_dirs()?.data_dir().to_path_buf()) }
pub fn rules_dir() -> Result<PathBuf> { Ok(data_dir()?.join("rules")) }
pub fn config_path() -> Result<PathBuf> { Ok(config_dir()?.join("config.toml")) }

pub fn ensure_project_dirs() -> Result<()> {
    fs::create_dir_all(config_dir()?)?;
    fs::create_dir_all(rules_dir()?)?;
    Ok(())
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("reading config: {}", path.display()))?;
    let cfg: Config = toml::from_str(&content)
        .with_context(|| format!("parsing config: {}", path.display()))?;
    Ok(cfg)
}

pub fn save_config(cfg: &Config) -> Result<()> {
    let path = config_path()?;
    let text = toml::to_string_pretty(cfg)?;
    fs::write(&path, text).with_context(|| format!("writing config: {}", path.display()))?;
    Ok(())
}


