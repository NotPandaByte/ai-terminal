use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};

use crate::config::{config_path, load_config, save_config};

pub fn run_setup_wizard() -> Result<()> {
    let theme = ColorfulTheme::default();
    let mut cfg = load_config()?;

    let api_base: String = Input::with_theme(&theme)
        .with_prompt("API base URL")
        .with_initial_text(cfg.api_base.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string()))
        .interact_text()?;
    cfg.api_base = Some(api_base);

    let api_key_env = std::env::var("OPENAI_API_KEY").ok();
    let initial_key = cfg.api_key.clone().or(api_key_env).unwrap_or_default();
    let api_key: String = Input::with_theme(&theme)
        .with_prompt("API key (stored locally)")
        .with_initial_text(initial_key)
        .interact_text()?;
    cfg.api_key = Some(api_key);

    let model: String = Input::with_theme(&theme)
        .with_prompt("Model")
        .with_initial_text(cfg.model.clone().unwrap_or_else(|| "gpt-4o-mini".to_string()))
        .interact_text()?;
    cfg.model = Some(model);

    save_config(&cfg)?;
    println!("Configuration saved at {}", config_path()?.display());
    Ok(())
}


