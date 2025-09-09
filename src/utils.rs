use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
use std::io::Read;

pub fn resolve_prompt(words: Vec<String>, is_agent: bool) -> Result<String> {
    if !words.is_empty() {
        return Ok(words.join(" "));
    }
    let mut buffer = String::new();
    let mut stdin = std::io::stdin();
    match stdin.read_to_string(&mut buffer) {
        Ok(n) if n > 0 => return Ok(buffer.trim().to_string()),
        _ => {}
    }
    let theme = ColorfulTheme::default();
    let label = if is_agent { "Enter agent objective" } else { "Enter prompt" };
    let input: String = Input::with_theme(&theme)
        .with_prompt(label)
        .interact_text()?;
    Ok(input)
}


