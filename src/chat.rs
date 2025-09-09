use anyhow::{anyhow, Context, Result};
use reqwest;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::config::load_config;

pub async fn call_chat_api(user_prompt: &str, rules_markdown: Option<&str>, is_agent: bool) -> Result<String> {
    let cfg = load_config()?;
    let api_key = cfg
        .api_key
        .clone()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .ok_or_else(|| anyhow!("API key not set. Run `ai setup`."))?;
    let api_base = cfg
        .api_base
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let model = cfg
        .model
        .clone()
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    #[derive(Serialize)]
    struct Message<'a> { role: &'a str, content: &'a str }
    #[derive(Serialize)]
    struct ChatReq<'a> { model: &'a str, messages: Vec<Message<'a>> }
    #[derive(Deserialize)]
    struct ChatResp { choices: Vec<Choice> }
    #[derive(Deserialize)]
    struct Choice { message: AssistantMessage }
    #[derive(Deserialize)]
    struct AssistantMessage { #[allow(dead_code)] role: String, content: String }

    let mut messages: Vec<Message> = Vec::new();
    let mut system_text = String::new();
    system_text.push_str("You are a helpful assistant in a terminal.\n");
    if is_agent {
        system_text.push_str("Act as an autonomous agent. Provide concise, actionable results.\n");
    }
    if let Some(rules) = rules_markdown {
        system_text.push_str("\n# Rules (Markdown)\n");
        system_text.push_str(rules);
    }
    messages.push(Message { role: "system", content: &system_text });
    messages.push(Message { role: "user", content: user_prompt });

    let body = ChatReq { model: &model, messages };
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("requesting chat completion")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("API error: {} - {}", status, text));
    }

    let parsed: ChatResp = resp.json().await.context("parsing chat response")?;
    let content = parsed
        .choices
        .get(0)
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "<no content>".to_string());
    Ok(content)
}

pub async fn stream_chat_api(user_prompt: &str, rules_markdown: Option<&str>, is_agent: bool) -> Result<()> {
    let cfg = load_config()?;
    let api_key = cfg
        .api_key
        .clone()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .ok_or_else(|| anyhow!("API key not set. Run `ai setup`."))?;
    let api_base = cfg
        .api_base
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let model = cfg
        .model
        .clone()
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    #[derive(Serialize)]
    struct Message<'a> { role: &'a str, content: &'a str }
    #[derive(Serialize)]
    struct ChatReq<'a> { model: &'a str, messages: Vec<Message<'a>>, stream: bool }

    let mut messages: Vec<Message> = Vec::new();
    let mut system_text = String::new();
    system_text.push_str("You are a helpful assistant in a terminal.\n");
    if is_agent {
        system_text.push_str("Act as an autonomous agent. Provide concise, actionable results.\n");
    }
    if let Some(rules) = rules_markdown {
        system_text.push_str("\n# Rules (Markdown)\n");
        system_text.push_str(rules);
    }
    messages.push(Message { role: "system", content: &system_text });
    messages.push(Message { role: "user", content: user_prompt });

    let body = ChatReq { model: &model, messages, stream: true };
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .context("requesting chat completion (stream)")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("API error: {} - {}", status, text));
    }

    let mut stream = resp.bytes_stream();
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let mut buffer = Vec::new();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        buffer.extend_from_slice(&bytes);
        while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
            let line = buffer.drain(..=pos).collect::<Vec<u8>>();
            // trim trailing newline/carriage return
            let line = line
                .into_iter()
                .filter(|b| *b != b'\n' && *b != b'\r')
                .collect::<Vec<u8>>();
            if line.starts_with(b"data:") {
                let data = &line[5..];
                let data = trim_ascii(data);
                if data == b"[DONE]" { continue; }
                if data.is_empty() { continue; }
                if let Ok(text) = std::str::from_utf8(data) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
                        if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
                            for choice in choices {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                        write!(handle, "{}", content)?;
                                        handle.flush()?;
                                    }
                                    if let Some(reasons) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                                        if reasons == "stop" {
                                            // completion ended
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    writeln!(handle)?;
    Ok(())
}

fn trim_ascii(s: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = s.len();
    while start < end && s[start].is_ascii_whitespace() { start += 1; }
    while end > start && s[end - 1].is_ascii_whitespace() { end -= 1; }
    &s[start..end]
}


