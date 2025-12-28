use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

use crate::config::load_config;

pub async fn show_status() -> Result<()> {
    let cfg = load_config()?;
    
    println!("--- Current Setup ---");
    
    // API Base
    let api_base = cfg.api_base.as_deref().unwrap_or("Not set (defaults to http://localhost:11434/v1)");
    println!("API Base: {}", api_base);
    
    // API Key
    let api_key = cfg.api_key.clone()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());
    if let Some(key) = api_key {
        let display_key = if key.len() > 8 {
            format!("{}...{}", &key[..4], &key[key.len()-4..])
        } else {
            "***".to_string()
        };
        println!("API Key: {} (configured)", display_key);
    } else {
        println!("API Key: Not set");
    }
    
    // Model
    let model = cfg.model.as_deref().unwrap_or("Not set");
    println!("Model: {}", model);
    
    // Check if local runner is running
    let is_local = cfg.api_base.as_ref()
        .map(|s| s.contains("localhost") || s.contains("127.0.0.1"))
        .unwrap_or(true); // Default to local if not set
    
    if is_local {
        let local_url = cfg.api_base.as_deref()
            .unwrap_or("http://localhost:11434/v1");
        let base_url = local_url.trim_end_matches("/v1");
        let health_url = format!("{}/api/tags", base_url);
        
        println!("\nLocal Runner Status:");
        match check_local_runner(&health_url).await {
            Ok(true) => {
                println!("  ✅ Ollama is running at {}", base_url);
                // Try to list available models
                if let Ok(models) = list_ollama_models(&health_url).await {
                    if !models.is_empty() {
                        println!("  Available models:");
                        for model in models {
                            println!("    - {}", model);
                        }
                    }
                }
            }
            Ok(false) => {
                println!("  ❌ Ollama is not responding at {}", base_url);
                println!("  Start it with: ollama serve");
            }
            Err(e) => {
                println!("  ❌ Error checking Ollama: {}", e);
            }
        }
    } else {
        println!("\nLocal Runner: Not configured (using cloud API)");
    }
    
    Ok(())
}

async fn check_local_runner(url: &str) -> Result<bool> {
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    
    match client.get(url).send().await {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

async fn list_ollama_models(url: &str) -> Result<Vec<String>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    
    let resp = client.get(url).send().await?;
    if resp.status().is_success() {
        let json: serde_json::Value = resp.json().await?;
        if let Some(models) = json["models"].as_array() {
            let model_names: Vec<String> = models
                .iter()
                .filter_map(|m| {
                    m["name"].as_str().map(|s| s.to_string())
                })
                .collect();
            return Ok(model_names);
        }
    }
    Ok(vec![])
}

