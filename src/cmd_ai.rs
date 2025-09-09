use anyhow::Result;

use crate::{chat::stream_chat_api, rules::read_active_ruleset, utils::resolve_prompt};

pub async fn run_ai(words: Vec<String>) -> Result<()> {
    let user_prompt = resolve_prompt(words, false)?;
    let rules = read_active_ruleset()?;
    stream_chat_api(&user_prompt, rules.as_deref(), false).await?;
    Ok(())
}


