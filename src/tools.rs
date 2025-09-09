use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, process::Command};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsContext {
    pub user: String,
    pub is_root: bool,
    pub shell: Option<String>,
    pub os_release: BTreeMap<String, String>,
    pub uname: String,
    pub detected_package_managers: Vec<String>,
    pub recommended_update_command: Option<String>,
}

pub fn gather_context() -> Result<ToolsContext> {
    let user = whoami::username();
    let is_root = nix::unistd::Uid::effective().is_root();
    let shell = std::env::var("SHELL").ok();

    let os_release = read_os_release();
    let uname = run_string(Command::new("uname").arg("-a"));

    let detected = detect_package_managers();
    let recommended_update_command = suggest_update_command(&detected, is_root);

    Ok(ToolsContext {
        user,
        is_root,
        shell,
        os_release,
        uname,
        detected_package_managers: detected,
        recommended_update_command,
    })
}

fn read_os_release() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if let Ok(text) = std::fs::read_to_string("/etc/os-release") {
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim_matches('"');
                map.insert(k.to_string(), v.to_string());
            }
        }
    }
    map
}

fn run_string(cmd: &mut Command) -> String {
    match cmd.output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

fn which(cmd: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let p = PathBuf::from(dir).join(cmd);
            if p.exists() { return Some(p); }
        }
    }
    None
}

pub fn detect_package_managers() -> Vec<String> {
    let candidates = vec![
        "pacman", "yay", "paru",
        "apt-get", "apt",
        "dnf", "zypper", "apk",
        "brew",
        "nix-env", "nixos-rebuild",
        "emerge",
    ];
    let mut found = Vec::new();
    for c in candidates {
        if which(c).is_some() { found.push(c.to_string()); }
    }
    found
}

fn suggest_update_command(detected: &Vec<String>, is_root: bool) -> Option<String> {
    let sudo = if is_root { "" } else { "sudo " };
    for m in detected {
        match m.as_str() {
            "yay" => return Some(format!("{}yay -Syu --noconfirm", if is_root { "" } else { "" })),
            "paru" => return Some(format!("{}paru -Syu --noconfirm", if is_root { "" } else { "" })),
            "pacman" => return Some(format!("{}pacman -Syu --noconfirm", sudo)),
            "apt-get" | "apt" => return Some(format!("{}apt update && {}apt upgrade -y", sudo, sudo)),
            "dnf" => return Some(format!("{}dnf upgrade -y", sudo)),
            "zypper" => return Some(format!("{}zypper refresh && {}zypper update -y", sudo, sudo)),
            "apk" => return Some(format!("{}apk update && {}apk upgrade", sudo, sudo)),
            "brew" => return Some("brew update && brew upgrade".to_string()),
            "nixos-rebuild" => return Some(format!("{}nixos-rebuild switch --upgrade", sudo)),
            "nix-env" => return Some("nix-env -u '*'".to_string()),
            "emerge" => return Some(format!("{}emerge --sync && {}emerge -uDU @world", sudo, sudo)),
            _ => {}
        }
    }
    None
}


