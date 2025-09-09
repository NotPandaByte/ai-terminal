use anyhow::Result;

use crate::{rules::read_active_ruleset, utils::resolve_prompt, tools};
use anyhow::Context;
use dialoguer::{theme::ColorfulTheme, Confirm};
// use std::process::Command; // no longer needed; kept commented for reference
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

pub async fn run_agent(words: Vec<String>) -> Result<()> {
    let objective = resolve_prompt(words, true)?;
    let rules = read_active_ruleset()?;

    // Gather context about the machine
    let ctx = tools::gather_context()?;

    // Safe reconnaissance to enrich planning context
    let recon = gather_recon();

    // Ask the model to return a plan as JSON describing commands to run
    // (removed unused AgentInput helper)
    let sys_rules = r#"You are a terminal ops agent. You can plan shell commands AND analyze files/codebases that were read during reconnaissance.

Output ONLY valid JSON with:
{
  "steps": [
    { "why": string, "cmd": string }
  ]
}
No markdown, no commentary outside JSON, no trailing commas.

IMPORTANT: For questions about files, codebases, or "what does this do", you have access to file contents in the Recon JSON. Use that information to answer directly in the "why" field, and only add shell commands if additional file reading is needed.

Constraints and behavior:
- For file/codebase analysis: Extract info from recon.files.* and explain in "why" fields. Only use commands if you need to read additional files.
- For system operations: Prefer minimal, safe, observable steps. Start read-only (list, status, dry-run) before mutating.
- Pick commands compatible with POSIX/fish. Avoid bashisms (process substitution, [[ ]], brace expansion needing bash).
- If a command needs root and context.is_root == false, prefix with "sudo ".
- Use package manager and update flow based on context.detected_package_managers and context.recommended_update_command if present.
- Be explicit and non-interactive: add flags like --yes/-y/--noconfirm where appropriate; include full paths or options to avoid prompts.
- Chain only when necessary. Prefer multiple small steps over one complex pipeline.
- Only include commands that exist on typical systems or likely present on Arch-based systems; if an optional tool is missing, show how to install it as a separate step.

Reference data provided:
- ToolsContext: context.user, context.is_root, context.shell, context.os_release, context.uname, context.detected_package_managers, context.recommended_update_command
- Recon: recon.system.* (system info), recon.files.* (file contents, directory listings, configs)

Common toolbelt (choose minimally necessary):
- System/process: ps, top/htop, pgrep, pidof, free -h, vmstat, uptime, dmesg
- Services/logs: systemctl, journalctl
- Files/disks: ls, cat, tail, head, grep, awk, sed, find, df -h, du -sh, lsblk, mount, tree
- Networking: ip a/r, ss -tulpn, ping, traceroute, dig/host, nslookup, curl, wget
- Firewall: ufw, firewall-cmd, iptables/nft (read-only first)
- Packages: pacman/yay/paru, apt, dnf, zypper, apk, brew, nix; use detected managers
- Containers (if present): docker, podman
- Hardware/system info: uname -a, lscpu, lsusb, lspci, sensors
- File analysis: cat, less, head, tail, grep, file, wc, find, tree

Workflow patterns:
- File/codebase questions: Use recon.files.* data to explain purpose, analyze configs, summarize code. Add file reading commands only if needed.
- Check processes: list, filter, explain, then optional action (kill, service restart) in separate steps.
- Check network: interfaces, routes, listening ports, connectivity (ping), DNS (dig), path (traceroute).
- Update system: prefer context.recommended_update_command; otherwise choose based on detected managers; sync/refresh first when needed.
- Inspect services: systemctl status <unit>; logs via journalctl -u <unit> -n 200 --no-pager.
- Disk issues: df -h; du -sh /var/log/*; identify top space hogs safely before removal.
- Security/firewall: read state first (ufw status, firewall-cmd --state, nft list ruleset); do not change without explicit objective.

Examples:
- "what does this codebase do?" -> analyze recon.files.readme_*, recon.files.config_main_*, explain purpose in "why"
- "what's in my hyprland config?" -> extract from recon.files.hyprland_config_*, summarize settings in "why"
- "show me my shell config" -> use recon.files.shell_config_* or add cat command if not present

Always produce the smallest set of steps to meet the objective. Keep "why" brief and actionable. The result MUST be valid JSON only."#;
    // (removed unused planning_prompt, now using enhanced_planning_prompt)

    // First phase: Ask what information is needed
    let info_request_prompt = format!(
        r#"You are a terminal ops agent. Given this objective, what specific system information do you need to gather before creating a plan?

Output ONLY valid JSON with:
{{
  "needed_info": [
    {{ "why": "explanation of why this info is needed", "cmd": "shell command to gather it" }}
  ],
  "has_enough_info": boolean
}}

If you have enough information from the provided context and recon to proceed, set has_enough_info to true and leave needed_info empty.

Objective: {}
Context (JSON): {}
Recon (JSON): {}
Rules (Markdown): {}
"#,
        objective,
        serde_json::to_string_pretty(&ctx).unwrap_or_default(),
        serde_json::to_string_pretty(&recon).unwrap_or_else(|_| "{}".to_string()),
        rules.as_deref().unwrap_or("")
    );

    let info_response = crate::chat::call_chat_api(&info_request_prompt, None, true).await?;
    
    #[derive(serde::Deserialize)]
    struct InfoRequest { why: String, cmd: String }
    #[derive(serde::Deserialize)]
    struct InfoNeeded { needed_info: Vec<InfoRequest>, has_enough_info: bool }
    
    let info_needed: InfoNeeded = serde_json::from_str(info_response.trim())
        .context("parsing info request JSON")?;

    // Gather additional information if needed
    let mut additional_info = serde_json::json!({});
    if !info_needed.has_enough_info && !info_needed.needed_info.is_empty() {
        println!("AI requests additional system information:");
        for (i, req) in info_needed.needed_info.iter().enumerate() {
            println!("{}. {} -> {}", i + 1, req.why, req.cmd);
        }
        
        let proceed_info = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Gather this information?")
            .default(true)
            .interact()?;
            
        if proceed_info {
            let additional_obj = additional_info.as_object_mut().unwrap();
            for (i, req) in info_needed.needed_info.iter().enumerate() {
                println!("\n[INFO {}] {}", i + 1, req.why);
                println!("$ {}", req.cmd);
                let output = run_capture(&req.cmd);
                let key = format!("info_{}", i + 1);
                additional_obj.insert(key, serde_json::json!({
                    "why": req.why,
                    "cmd": req.cmd,
                    "output": if output.len() > 8192 { 
                        format!("{}... [truncated at 8192 chars]", &output[..8192])
                    } else { 
                        output 
                    }
                }));
            }
        }
    }

    // Second phase: Create the actual plan with all gathered information
    let enhanced_planning_prompt = format!(
        "{}\n\nGiven the following objective, machine context, reconnaissance, additional gathered info, and rules, produce the minimal safe steps.\nObjective: {}\nContext (JSON): {}\nRecon (JSON): {}\nAdditional Info (JSON): {}\nRules (Markdown):\n{}\n",
        sys_rules,
        objective,
        serde_json::to_string_pretty(&ctx).unwrap_or_default(),
        serde_json::to_string_pretty(&recon).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string_pretty(&additional_info).unwrap_or_else(|_| "{}".to_string()),
        rules.as_deref().unwrap_or("")
    );

    let plan_text = crate::chat::call_chat_api(&enhanced_planning_prompt, None, true).await?;
    #[derive(serde::Deserialize)]
    struct Step { why: String, cmd: String }
    #[derive(serde::Deserialize)]
    struct Plan { steps: Vec<Step> }
    let plan: Plan = serde_json::from_str(plan_text.trim()).context("parsing agent plan JSON")?;

    // Show plan and ask confirmation
    let theme = ColorfulTheme::default();
    println!("Planned steps:");
    for (i, s) in plan.steps.iter().enumerate() {
        println!("{}. {}\n   $ {}", i + 1, s.why, s.cmd);
    }
    let proceed = Confirm::with_theme(&theme)
        .with_prompt("Execute these commands?")
        .default(false)
        .interact()?;
    if !proceed { return Ok(()); }

    // Execute sequentially, streaming stdout/stderr per command with timestamps
    let total = plan.steps.len();
    for (i, s) in plan.steps.into_iter().enumerate() {
        println!("\n[{} / {}] {}", i + 1, total, s.why);
        println!("$ {}", s.cmd);
        let code = stream_command(&s.cmd).context("running planned command")?;
        if code != 0 {
            println!("Step exited with code {}. Stopping.", code);
            break;
        }
    }

    Ok(())
}



fn gather_recon() -> serde_json::Value {
    use serde_json::json;

    fn cap(cmd: &str) -> String {
        let out = run_capture(cmd);
        let max = 16_384; // 16 KB cap
        if out.len() > max { out[..max].to_string() } else { out }
    }

    fn read_file_safe(path: &str) -> String {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let max = 8_192; // 8 KB cap for files
                if content.len() > max { 
                    format!("{}... [truncated at {} chars]", &content[..max], max)
                } else { 
                    content 
                }
            }
            Err(_) => String::new(),
        }
    }

    let mut recon = json!({
        "system": {
            "uname": cap("uname -a"),
            "uptime": cap("uptime"),
            "memory": cap("free -h"),
            "disk": cap("df -h"),
            "interfaces": cap("ip -brief a"),
            "routes": cap("ip r"),
            "listening": cap("ss -tulpn | head -n 50"),
            "services_sample": cap("systemctl list-units --type=service --state=running | head -n 50"),
            "recent_kernel_msgs": cap("dmesg -T | tail -n 50"),
        },
        "files": {
            "current_dir": cap("pwd"),
            "dir_listing": cap("ls -la"),
            "tree_sample": cap("find . -maxdepth 3 -type f | head -n 100"),
        }
    });

    // Try to read common important files if they exist
    let important_files = vec![
        ("readme", vec!["README.md", "README.txt", "README", "readme.md"]),
        ("config_main", vec!["Cargo.toml", "package.json", "pyproject.toml", "setup.py", "Makefile", "CMakeLists.txt"]),
        ("hyprland_config", vec!["~/.config/hypr/hyprland.conf", "~/.config/hypr/hyprland.config"]),
        ("shell_config", vec!["~/.bashrc", "~/.zshrc", "~/.config/fish/config.fish"]),
        ("git_info", vec![".git/config", ".gitignore"]),
    ];

    let files_obj = recon["files"].as_object_mut().unwrap();
    
    for (category, paths) in important_files {
        for path in paths {
            let expanded_path = if path.starts_with("~/") {
                let home = std::env::var("HOME").unwrap_or_default();
                path.replace("~/", &format!("{}/", home))
            } else {
                path.to_string()
            };
            
            let content = read_file_safe(&expanded_path);
            if !content.is_empty() {
                files_obj.insert(format!("{}_{}", category, path.replace("/", "_").replace(".", "_")), json!(content));
                break; // Only read first found file in each category
            }
        }
    }

    recon
}

fn run_capture(cmd: &str) -> String {
    use std::process::Command;
    match Command::new("/usr/bin/fish")
        .arg("-lc")
        .arg(cmd)
        .output()
    {
        Ok(out) => {
            let mut s = String::from_utf8_lossy(&out.stdout).to_string();
            if s.trim().is_empty() {
                s = String::from_utf8_lossy(&out.stderr).to_string();
            }
            s.trim().to_string()
        }
        Err(_) => String::new(),
    }
}

fn stream_command(cmd: &str) -> anyhow::Result<i32> {
    let mut child = TokioCommand::new("/usr/bin/fish")
        .arg("-lc")
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawning command")?;

    let stdout = child.stdout.take().context("taking stdout")?;
    let stderr = child.stderr.take().context("taking stderr")?;

    let start = Instant::now();

    // Run inside current runtime
    tokio::runtime::Handle::current().block_on(async move {
        let mut out_lines = BufReader::new(stdout).lines();
        let mut err_lines = BufReader::new(stderr).lines();
        let mut out_done = false;
        let mut err_done = false;
        while !(out_done && err_done) {
            tokio::select! {
                line = out_lines.next_line(), if !out_done => {
                    match line.transpose() {
                        Some(Ok(l)) => {
                            let t = start.elapsed().as_secs_f32();
                            println!("[+{:.3}s] [OUT] {}", t, l);
                        }
                        Some(Err(_)) => { out_done = true; }
                        None => { out_done = true; }
                    }
                }
                line = err_lines.next_line(), if !err_done => {
                    match line.transpose() {
                        Some(Ok(l)) => {
                            let t = start.elapsed().as_secs_f32();
                            println!("[+{:.3}s] [ERR] {}", t, l);
                        }
                        Some(Err(_)) => { err_done = true; }
                        None => { err_done = true; }
                    }
                }
            }
        }
        let status = child.wait().await.context("waiting for command")?;
        anyhow::Ok(status.code().unwrap_or_else(|| if status.success() { 0 } else { 1 }))
    })
}

