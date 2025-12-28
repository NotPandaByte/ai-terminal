use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::{self, Client, header::{ACCEPT, CONTENT_TYPE}};
use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::config::load_config;

// --- Shared Types ---
#[derive(Serialize)]
struct Message<'a> { role: &'a str, content: &'a str }

#[derive(Serialize)]
struct ChatReq<'a> { 
    model: &'a str, 
    messages: Vec<Message<'a>>, 
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool> 
}

#[derive(Deserialize)]
struct ChatResp { choices: Vec<Choice> }
#[derive(Deserialize)]
struct Choice { message: AssistantMessage }
#[derive(Deserialize)]
struct AssistantMessage { content: String }

/// Helper to prepare common components
fn prepare_request_data(user_prompt: &str, rules_markdown: Option<&str>, is_agent: bool) -> Result<(String, String, String, String)> {
    let cfg = load_config()?;
    
    // Default to local Ollama if no base is provided, or use config
    let api_base = cfg.api_base.clone().unwrap_or_else(|| "http://localhost:11434/v1".to_string());
    let model = cfg.model.clone().unwrap_or_else(|| "llama3".to_string());
    
    // API key is now optional for local providers
    let api_key = cfg.api_key.clone()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .unwrap_or_else(|| "no-key-required".to_string());

    let mut system_text = String::from("You are a helpful assistant in a terminal.\n");
    if is_agent {
        system_text.push_str("Act as an autonomous agent. Provide concise, actionable results.\n");
    }
    if let Some(rules) = rules_markdown {
        system_text.push_str(&format!("\n# Rules (Markdown)\n{}", rules));
    }

    Ok((api_base, api_key, model, system_text))
}

pub async fn call_chat_api(user_prompt: &str, rules_markdown: Option<&str>, is_agent: bool) -> Result<String> {
    let (api_base, api_key, model, system_prompt) = prepare_request_data(user_prompt, rules_markdown, is_agent)?;
    
    let messages = vec![
        Message { role: "system", content: &system_prompt },
        Message { role: "user", content: user_prompt },
    ];

    let body = ChatReq { model: &model, messages, stream: None };
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    let resp = Client::new()
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("Requesting chat completion")?;

    if !resp.status().is_success() {
        return Err(anyhow!("API error: {} - {}", resp.status(), resp.text().await?));
    }

    let parsed: ChatResp = resp.json().await?;
    Ok(parsed.choices.get(0).map(|c| c.message.content.clone()).unwrap_or_default())
}

pub async fn stream_chat_api(user_prompt: &str, rules_markdown: Option<&str>, is_agent: bool) -> Result<()> {
    let (api_base, api_key, model, system_prompt) = prepare_request_data(user_prompt, rules_markdown, is_agent)?;

    let messages = vec![
        Message { role: "system", content: &system_prompt },
        Message { role: "user", content: user_prompt },
    ];

    let body = ChatReq { model: &model, messages, stream: Some(true) };
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    let resp = Client::new()
        .post(url)
        .bearer_auth(api_key)
        .header(ACCEPT, "text/event-stream")
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .context("Requesting stream")?;

    let mut stream = resp.bytes_stream();
    let mut stdout = std::io::stdout().lock();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        let text = String::from_utf8_lossy(&bytes);
        
        for line in text.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" { break; }
                
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = val["choices"][0]["delta"]["content"].as_str() {
                        write!(stdout, "{}", content)?;
                        stdout.flush()?;
                    }
                }
            }
        }
    }
    writeln!(stdout)?;
    Ok(())
}
