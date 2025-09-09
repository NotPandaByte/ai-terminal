use anyhow::{anyhow, Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::{fs, path::Path, process::Command};

use crate::config::{load_config, rules_dir, save_config};

pub fn list_rulesets() -> Result<Vec<String>> {
    let mut items = Vec::new();
    let dir = rules_dir()?;
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(name);
            }
        }
    }
    items.sort();
    Ok(items)
}

pub fn read_ruleset_content(filename: &str) -> Result<String> {
    let path = rules_dir()?.join(filename);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("reading ruleset: {}", path.display()))?;
    Ok(content)
}

pub fn read_active_ruleset() -> Result<Option<String>> {
    let cfg = load_config()?;
    if let Some(name) = cfg.active_ruleset {
        let content = read_ruleset_content(&name)?;
        return Ok(Some(content));
    }
    Ok(None)
}

pub fn run_rules_picker() -> Result<()> {
    let theme = ColorfulTheme::default();
    loop {
        let options = vec![
            "Create new ruleset",
            "Edit active ruleset",
            "Choose active ruleset",
            "Exit",
        ];
        let idx = Select::with_theme(&theme)
            .with_prompt("Rules")
            .items(&options)
            .default(0)
            .interact()?;
        match idx {
            0 => create_new_ruleset()?,
            1 => edit_active_ruleset()?,
            2 => choose_active_ruleset()?,
            _ => break,
        }
    }
    Ok(())
}

fn create_new_ruleset() -> Result<()> {
    let theme = ColorfulTheme::default();
    let name: String = Input::with_theme(&theme)
        .with_prompt("New ruleset name (e.g. default.md)")
        .with_initial_text("rules.md")
        .interact_text()?;
    let path = rules_dir()?.join(&name);
    if path.exists() {
        let overwrite = Confirm::with_theme(&theme)
            .with_prompt("File exists. Overwrite?")
            .default(false)
            .interact()?;
        if !overwrite { return Ok(()); }
    }
    fs::write(&path, "# Rules\n\n- Add your rules here.\n")?;
    open_in_editor(&path)?;
    let set_active = Confirm::with_theme(&theme)
        .with_prompt("Set this ruleset as active?")
        .default(true)
        .interact()?;
    if set_active {
        let mut cfg = load_config()?;
        cfg.active_ruleset = Some(name);
        save_config(&cfg)?;
        println!("Active ruleset updated.");
    }
    Ok(())
}

fn edit_active_ruleset() -> Result<()> {
    let cfg = load_config()?;
    let Some(active) = cfg.active_ruleset else {
        println!("No active ruleset. Choose or create one first.");
        return Ok(());
    };
    let path = rules_dir()?.join(active);
    open_in_editor(&path)?;
    Ok(())
}

fn choose_active_ruleset() -> Result<()> {
    let theme = ColorfulTheme::default();
    let items = list_rulesets()?;
    if items.is_empty() {
        println!("No rulesets found. Create one first.");
        return Ok(());
    }
    let idx = Select::with_theme(&theme)
        .with_prompt("Choose active ruleset")
        .items(&items)
        .default(0)
        .interact()?;
    let selection = items[idx].clone();
    let mut cfg = load_config()?;
    cfg.active_ruleset = Some(selection);
    save_config(&cfg)?;
    println!("Active ruleset updated.");
    Ok(())
}

fn open_in_editor(path: &Path) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let status = Command::new(editor)
        .arg(path)
        .status()
        .with_context(|| format!("launching editor for {}", path.display()))?;
    if !status.success() {
        return Err(anyhow!("editor exited with status {:?}", status.code()));
    }
    Ok(())
}


