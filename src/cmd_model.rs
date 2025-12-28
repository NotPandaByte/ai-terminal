use anyhow::{anyhow, Context, Result};
use atty::Stream;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::process::Command;

use crate::config::{load_config, save_config};

/// Maps display names to actual Ollama model names
/// Based on Ollama's model library naming conventions
fn map_display_to_model(display_name: &str) -> String {
    match display_name {
        // Llama models
        "llama3 (8B)" => "llama3".to_string(),
        "llama3 (70B)" => "llama3:70b".to_string(),
        "llama3.2 (8B)" => "llama3.2".to_string(),
        "llama3.2 (70B)" => "llama3.2:70b".to_string(),
        // Mistral models
        "mistral (7B)" => "mistral".to_string(),
        "mistral-medium (8x22B)" => "mistral-medium".to_string(),
        "mistral-small (8x7B)" => "mistral-small".to_string(),
        // Mixtral models
        "mixtral (8x7B)" => "mixtral".to_string(),
        "mixtral (8x22B)" => "mixtral:8x22b".to_string(),
        // Phi models
        "phi3 (4B)" => "phi3".to_string(),
        "phi (2.7B)" => "phi".to_string(),
        "phi2 (2.7B)" => "phi2".to_string(),
        // Dolphin models
        "dolphin-2.9 (70B)" => "dolphin-mixtral".to_string(),
        "dolphin-phi (2.7B)" => "dolphin-phi".to_string(),
        // Other models
        "zephyr (7B)" => "zephyr".to_string(),
        "tinyllama (1.1B)" => "tinyllama".to_string(),
        "wizardlm (7B)" => "wizardlm2".to_string(),
        "openhermes (2.5B)" => "openhermes".to_string(),
        "openhermes (7B)" => "openhermes:7b".to_string(),
        "orca-mini (3B)" => "orca-mini".to_string(),
        "orca-mini (7B)" => "orca-mini:7b".to_string(),
        "yarn-llama-2 (7B)" => "yarn-llama2:7b".to_string(),
        "yarn-mistral (7B)" => "yarn-mistral:7b".to_string(),
        // CodeLlama models
        "codellama (7B)" => "codellama".to_string(),
        "codellama (13B)" => "codellama:13b".to_string(),
        "codellama (34B)" => "codellama:34b".to_string(),
        // Vicuna models
        "vicuna (7B)" => "vicuna".to_string(),
        "vicuna (13B)" => "vicuna:13b".to_string(),
        // Other specialized models
        "stablelm-zephyr (3B)" => "stablelm-zephyr-3b".to_string(),
        "nous-hermes-2 (10.7B)" => "nous-hermes2".to_string(),
        "deepseek (67B)" => "deepseek".to_string(),
        "deepseek-coder (6.7B)" => "deepseek-coder".to_string(),
        "deepseek-coder (33B)" => "deepseek-coder:33b".to_string(),
        "openchat (3.5B)" => "openchat".to_string(),
        "openchat (7B)" => "openchat:7b".to_string(),
        "command-r (35B)" => "command-r".to_string(),
        "gemma (2B)" => "gemma:2b".to_string(),
        "gemma (7B)" => "gemma:7b".to_string(),
        "neural-chat (7B)" => "neural-chat".to_string(),
        "hermes-2-pro (7B)" => "nous-hermes2".to_string(),
        "hermes-2-pro (13B)" => "nous-hermes2:13b".to_string(),
        "starling (7B)" => "starling-lm".to_string(),
        // If it's not a display name, return as-is (for cloud models or custom)
        _ => display_name.to_string(),
    }
}

/// Check if a model is installed in Ollama
fn is_model_installed(model_name: &str) -> bool {
    let output = Command::new("ollama")
        .arg("list")
        .output();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Extract base name (without tag) for comparison
        let base_name = model_name.split(':').next().unwrap_or(model_name);
        
        // Skip header line and check each model
        stdout.lines().skip(1).any(|line| {
            if let Some(name_col) = line.split_whitespace().next() {
                // Check for exact match or if the listed model starts with our base name
                name_col == model_name || name_col.starts_with(&format!("{}:", base_name)) || name_col == base_name
            } else {
                false
            }
        })
    } else {
        false
    }
}

/// Install a model using ollama pull
fn install_model(model_name: &str) -> Result<()> {
    println!("📥 Installing model '{}'...", model_name);
    println!("This may take a while depending on model size...");
    
    let status = Command::new("ollama")
        .arg("pull")
        .arg(model_name)
        .status()
        .context("Failed to run ollama pull. Is Ollama installed and running?")?;
    
    if status.success() {
        println!("✅ Model '{}' installed successfully!", model_name);
        Ok(())
    } else {
        Err(anyhow!("Failed to install model '{}'. Exit code: {:?}", model_name, status.code()))
    }
}

/// Check if we're using a local Ollama instance
fn is_local_ollama() -> bool {
    if let Ok(cfg) = load_config() {
        if let Some(api_base) = &cfg.api_base {
            return api_base.contains("localhost") || api_base.contains("127.0.0.1");
        }
    }
    // Default to local if not configured
    true
}

/// Maps actual model names back to display names for matching
fn map_model_to_display(model_name: &str) -> String {
    // Check if it matches any of our mapped models (exact match or base name match)
    let model_lower = model_name.to_lowercase();
    for (display, actual) in [
        ("llama3 (8B)", "llama3"),
        ("llama3 (70B)", "llama3:70b"),
        ("llama3.2 (8B)", "llama3.2"),
        ("llama3.2 (70B)", "llama3.2:70b"),
        ("mistral (7B)", "mistral"),
        ("mistral-medium (8x22B)", "mistral-medium"),
        ("mistral-small (8x7B)", "mistral-small"),
        ("mixtral (8x7B)", "mixtral"),
        ("mixtral (8x22B)", "mixtral:8x22b"),
        ("phi3 (4B)", "phi3"),
        ("phi (2.7B)", "phi"),
        ("phi2 (2.7B)", "phi2"),
        ("dolphin-2.9 (70B)", "dolphin-mixtral"),
        ("dolphin-phi (2.7B)", "dolphin-phi"),
        ("zephyr (7B)", "zephyr"),
        ("tinyllama (1.1B)", "tinyllama"),
        ("wizardlm (7B)", "wizardlm2"),
        ("openhermes (2.5B)", "openhermes"),
        ("openhermes (7B)", "openhermes:7b"),
        ("orca-mini (3B)", "orca-mini"),
        ("orca-mini (7B)", "orca-mini:7b"),
        ("yarn-llama-2 (7B)", "yarn-llama2:7b"),
        ("yarn-mistral (7B)", "yarn-mistral:7b"),
        ("codellama (7B)", "codellama"),
        ("codellama (13B)", "codellama:13b"),
        ("codellama (34B)", "codellama:34b"),
        ("vicuna (7B)", "vicuna"),
        ("vicuna (13B)", "vicuna:13b"),
        ("stablelm-zephyr (3B)", "stablelm-zephyr-3b"),
        ("nous-hermes-2 (10.7B)", "nous-hermes2"),
        ("deepseek (67B)", "deepseek"),
        ("deepseek-coder (6.7B)", "deepseek-coder"),
        ("deepseek-coder (33B)", "deepseek-coder:33b"),
        ("openchat (3.5B)", "openchat"),
        ("openchat (7B)", "openchat:7b"),
        ("command-r (35B)", "command-r"),
        ("gemma (2B)", "gemma:2b"),
        ("gemma (7B)", "gemma:7b"),
        ("neural-chat (7B)", "neural-chat"),
        ("hermes-2-pro (7B)", "nous-hermes2"),
        ("hermes-2-pro (13B)", "nous-hermes2:13b"),
        ("starling (7B)", "starling-lm"),
    ] {
        let actual_lower = actual.to_lowercase();
        if model_lower == actual_lower || model_lower.starts_with(&format!("{}:", actual_lower)) {
            return display.to_string();
        }
    }
    model_name.to_string()
}

pub fn run_set_model(name: Option<String>) -> Result<()> {
    let theme = ColorfulTheme::default();
    if let Some(n) = name {
        let trimmed = n.trim().to_string();
        if !trimmed.is_empty() {
            // Check if this is a local model that needs installation
            if is_local_ollama() {
                let is_cloud_model = trimmed.starts_with("gpt-") 
                    || trimmed == "o1" 
                    || trimmed.starts_with("claude-");
                
                if !is_cloud_model && !is_model_installed(&trimmed) {
                    println!("\n⚠️  Model '{}' is not installed in Ollama.", trimmed);
                    let install = Confirm::with_theme(&theme)
                        .with_prompt("Would you like to install it now?")
                        .default(true)
                        .interact()?;
                    
                    if install {
                        install_model(&trimmed)?;
                    } else {
                        println!("⚠️  Model not installed. You may need to install it manually with: ollama pull {}", trimmed);
                    }
                }
            }
            
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
        // GPT-4 family
        "gpt-4o-mini",
        "gpt-4o",
        "gpt-4-turbo",
        "gpt-4",
        // GPT-3.5
        "gpt-3.5-turbo",
        // Other providers
        "o1",
        "claude-3.5-sonnet",
        // Local models (Ollama)
        // Local models (Ollama, LM Studio, etc.)
        "llama3 (8B)",
        "llama3 (70B)",
        "llama3.2 (8B)",
        "llama3.2 (70B)",
        "mistral (7B)",
        "mistral-medium (8x22B)",
        "mistral-small (8x7B)",
        "mixtral (8x7B)",
        "mixtral (8x22B)",
        "phi3 (4B)",
        "phi (2.7B)",
        "phi2 (2.7B)",
        "dolphin-2.9 (70B)",
        "dolphin-phi (2.7B)",
        "zephyr (7B)",
        "tinyllama (1.1B)",
        "wizardlm (7B)",
        "openhermes (2.5B)",
        "openhermes (7B)",
        "orca-mini (3B)",
        "orca-mini (7B)",
        "yarn-llama-2 (7B)",
        "yarn-mistral (7B)",
        "codellama (7B)",
        "codellama (13B)",
        "codellama (34B)",
        "vicuna (7B)",
        "vicuna (13B)",
        "stablelm-zephyr (3B)",
        "nous-hermes-2 (10.7B)",
        "deepseek (67B)",
        "deepseek-coder (6.7B)",
        "deepseek-coder (33B)",
        "openchat (3.5B)",
        "openchat (7B)",
        "command-r (35B)",
        "gemma (2B)",
        "gemma (7B)",
        "neural-chat (7B)",
        "hermes-2-pro (7B)",
        "hermes-2-pro (13B)",
        "starling (7B)",
        "Custom...",
    ];

    let current = load_config()?.model.unwrap_or_else(|| "gpt-4o-mini".to_string());
    // Try to match current model (could be display name or actual model name)
    let current_display = map_model_to_display(&current);
    let default_index = common_models
        .iter()
        .position(|m| *m == current || *m == current_display)
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
        // Map display name to actual model name
        map_display_to_model(selected)
    };

    // Check if this is a local model and if it needs to be installed
    if is_local_ollama() {
        // Check if it's a cloud model (starts with gpt-, o1, claude-)
        let is_cloud_model = final_model.starts_with("gpt-") 
            || final_model == "o1" 
            || final_model.starts_with("claude-");
        
        if !is_cloud_model && !is_model_installed(&final_model) {
            println!("\n⚠️  Model '{}' is not installed in Ollama.", final_model);
            let install = Confirm::with_theme(&theme)
                .with_prompt("Would you like to install it now?")
                .default(true)
                .interact()?;
            
            if install {
                install_model(&final_model)?;
            } else {
                println!("⚠️  Model not installed. You may need to install it manually with: ollama pull {}", final_model);
            }
        }
    }

    let mut cfg = load_config()?;
    cfg.model = Some(final_model.clone());
    save_config(&cfg)?;
    println!("Model set to {}", final_model);
    Ok(())
}


