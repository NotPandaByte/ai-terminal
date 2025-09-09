use anyhow::{anyhow, Result};
use atty::Stream;
use dialoguer::{theme::ColorfulTheme, Input, Select};

use crate::config::{load_config, save_config};

pub fn run_set_model(name: Option<String>) -> Result<()> {
    let theme = ColorfulTheme::default();
    if let Some(n) = name {
        let trimmed = n.trim().to_string();
        if !trimmed.is_empty() {
            let mut cfg = load_config()?;
            cfg.model = Some(trimmed.clone());
            save_config(&cfg)?;
            println!("Model set to {}", trimmed);
            return Ok(());
        }
    }

    if !(atty::is(Stream::Stdin) && atty::is(Stream::Stdout)) {
        return Err(anyhow!(
            "interactive picker not available; run in a TTY or pass a model name"
        ));
    }

    let common_models = vec![
        // GPT-5 family
        "gpt-5.1",
        "gpt-5.1-mini",
        "gpt-5o",
        "gpt-5o-mini",
        "gpt-5-turbo",
        "gpt-5",
        // GPT-4 family
        "gpt-4o-mini",
        "gpt-4o",
        "gpt-4.1-mini",
        "gpt-4.1",
        "gpt-4-turbo",
        "gpt-4",
        "gpt-3.5-turbo",
        // Other providers
        "o3-mini",
        "o1",
        "llama-3.1-405b",
        "llama-3.1-70b",
        "llama-3.1-8b",
        "mixtral-8x7b",
        "claude-3.5-sonnet",
        "Custom...",
    ];

    let current = load_config()?.model.unwrap_or_else(|| "gpt-4o-mini".to_string());
    let default_index = common_models
        .iter()
        .position(|m| *m == current)
        .unwrap_or(0)
        .min(common_models.len().saturating_sub(1));

    let idx = Select::with_theme(&theme)
        .with_prompt("Choose a model")
        .items(&common_models)
        .default(default_index)
        .interact()?;

    let selected = common_models[idx];
    let final_model = if selected == "Custom..." {
        Input::with_theme(&theme)
            .with_prompt("Enter custom model name")
            .with_initial_text(current)
            .interact_text()?
    } else {
        selected.to_string()
    };

    let mut cfg = load_config()?;
    cfg.model = Some(final_model.clone());
    save_config(&cfg)?;
    println!("Model set to {}", final_model);
    Ok(())
}


