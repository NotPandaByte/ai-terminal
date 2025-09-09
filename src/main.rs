use anyhow::Result;
use clap::{Parser, Subcommand};

mod config;
mod utils;
mod chat;
mod rules;
mod cmd_ai;
mod cmd_agent;
mod cmd_model;
mod cmd_setup;
mod tools;

#[derive(Parser, Debug)]
#[command(name="ai", version)]
struct Cli {
    /// Positional prompt for one-shot mode. If omitted, prompts for input.
    prompt: Vec<String>,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Agent mode with tools
    Agent {
        /// Objective or prompt for the agent
        prompt: Vec<String>,
    },
    /// Manage rule markdown files and active set
    Rules,
    /// Configure APIs, model, endpoints
    Setup,
    /// Set or change the active model
    Model {
        /// Optional model name (if omitted, you will be prompted)
        name: Option<String>,
    },
}

// moved to crate::config

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    config::ensure_project_dirs()?;

    match &cli.cmd {
        Some(Cmd::Agent { prompt }) => {
            cmd_agent::run_agent(prompt.clone()).await?;
        }
        Some(Cmd::Rules) => {
            rules::run_rules_picker()?;
        }
        Some(Cmd::Setup) => {
            cmd_setup::run_setup_wizard()?;
        }
        Some(Cmd::Model { name }) => {
            cmd_model::run_set_model(name.clone())?;
        }
        None => {
            cmd_ai::run_ai(cli.prompt.clone()).await?;
        }
    }

    Ok(())
}

// All functions moved to their respective modules (utils.rs, rules.rs, cmd_setup.rs, cmd_model.rs, chat.rs)
