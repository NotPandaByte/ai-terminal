use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input, Select};

use crate::config::{config_path, load_config, save_config};

pub fn run_setup_wizard() -> Result<()> {
    let theme = ColorfulTheme::default();
    let mut cfg = load_config()?;

    println!("--- AI Terminal Setup ---");

    // 1. Choose Provider (Quick Presets)
    let providers = vec!["OpenAI (Cloud)", "Ollama (Local)", "Custom"];
    let selection = Select::with_theme(&theme)
        .with_prompt("Select your backend provider")
        .default(0)
        .items(&providers)
        .interact()?;

    match selection {
        0 => { // OpenAI
            cfg.api_base = Some("https://api.openai.com/v1".to_string());
            if cfg.model.is_none() { cfg.model = Some("gpt-4o-mini".to_string()); }
        }
        1 => { // Ollama
            cfg.api_base = Some("http://localhost:11434/v1".to_string());
            if cfg.model.is_none() || cfg.model.as_deref() == Some("gpt-4o-mini") { 
                cfg.model = Some("llama3".to_string()); 
            }
            cfg.api_key = Some("ollama".to_string()); // Placeholder
        }
        _ => {} // Keep existing or custom
    }

    // 2. Fine-tune the API Base
    let api_base: String = Input::with_theme(&theme)
        .with_prompt("Confirm API base URL")
        .with_initial_text(cfg.api_base.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string()))
        .interact_text()?;
    cfg.api_base = Some(api_base);

    // 3. Handle API Key (make it optional for local use)
    let api_key_env = std::env::var("OPENAI_API_KEY").ok();
    let initial_key = cfg.api_key.clone().or(api_key_env).unwrap_or_default();
    
    let is_local = cfg.api_base.as_ref().map(|s| s.contains("localhost") || s.contains("127.0.0.1")).unwrap_or(false);
    
    let key_prompt = if is_local { "API key (Optional for local)" } else { "API key" };
    
    let api_key: String = Input::with_theme(&theme)
        .with_prompt(key_prompt)
        .allow_empty(true)
        .with_initial_text(initial_key)
        .interact_text()?;
    cfg.api_key = if api_key.is_empty() { None } else { Some(api_key) };

    // 4. Set Model
    let model: String = Input::with_theme(&theme)
        .with_prompt("Model name")
        .with_initial_text(cfg.model.clone().unwrap_or_else(|| "gpt-4o-mini".to_string()))
        .interact_text()?;
    cfg.model = Some(model);

    save_config(&cfg)?;
    println!("\n✅ Configuration saved at {}", config_path()?.display());
    println!("Try it out by running: ai \"Hello!\"");
    
    Ok(())
}
