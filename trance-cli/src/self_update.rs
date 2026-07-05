// SPDX-License-Identifier: MIT

use std::process::Command;

use anyhow::Result;

fn is_command_available(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for p in path.split(':') {
            let p_path = std::path::Path::new(p).join(cmd);
            if p_path.exists() {
                return true;
            }
        }
    }
    false
}

#[tracing::instrument]
fn handle_apt_update() -> Result<()> {
    println!("Checking for updates in APT repository...");

    let output = Command::new("apt-cache")
        .args(["policy", "trance"])
        .output();

    match output {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut installed = None;
            let mut candidate = None;
            for line in text.lines() {
                let line = line.trim();
                if let Some(stripped) = line.strip_prefix("Installed:") {
                    installed = Some(stripped.trim().to_string());
                } else if let Some(stripped) = line.strip_prefix("Candidate:") {
                    candidate = Some(stripped.trim().to_string());
                }
            }

            match (installed, candidate) {
                (Some(inst), Some(cand)) => {
                    if inst == "(none)" {
                        println!(" [!] Trance is not currently installed as a system package.");
                        println!("     -> Fix: Install it with: sudo apt install trance");
                    } else if inst != cand {
                        println!(" [!] A new version is available: {inst} -> {cand}");
                        println!(
                            "     -> Run: sudo apt update && sudo apt install --only-upgrade trance"
                        );
                    } else {
                        println!(" [✔] Trance is already up to date (version {inst}).");
                    }
                }
                _ => {
                    println!(" [!] Could not determine installed package policy for 'trance'.");
                    println!("     -> Note: Ensure the UberMetroid APT repository is registered.");
                }
            }
        }
        Err(_) => {
            println!(" [!] 'apt-cache' command failed.");
        }
    }
    Ok(())
}

#[tracing::instrument]
fn handle_dnf_update() -> Result<()> {
    println!("Checking for updates in DNF repository...");

    let output = Command::new("dnf")
        .args(["list", "--cacheonly", "trance"])
        .output();

    match output {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut installed = None;
            let mut candidate = None;
            let mut section = "";
            for line in text.lines() {
                let line = line.trim();
                if line.contains("Installed Packages") {
                    section = "installed";
                } else if line.contains("Available Packages") {
                    section = "available";
                } else if line.starts_with("trance.") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let version = parts[1].to_string();
                        if section == "installed" {
                            installed = Some(version);
                        } else if section == "available" {
                            candidate = Some(version);
                        }
                    }
                }
            }

            match (installed, candidate) {
                (Some(inst), Some(cand)) => {
                    if inst != cand {
                        println!(" [!] A new version is available: {inst} -> {cand}");
                        println!("     -> Run: sudo dnf upgrade trance");
                    } else {
                        println!(" [✔] Trance is already up to date (version {inst}).");
                    }
                }
                (None, Some(_)) => {
                    println!(" [!] Trance is not currently installed as a system package.");
                    println!("     -> Fix: Install it with: sudo dnf install trance");
                }
                _ => {
                    println!(" [!] Could not determine installed package policy for 'trance'.");
                    println!("     -> Note: Ensure the UberMetroid DNF repository is registered.");
                }
            }
        }
        Err(_) => {
            println!(" [!] 'dnf' command failed.");
        }
    }
    Ok(())
}

#[tracing::instrument]
pub fn handle_self_update() -> Result<()> {
    if is_command_available("apt-cache") {
        handle_apt_update()
    } else if is_command_available("dnf") {
        handle_dnf_update()
    } else {
        println!(" [!] Neither 'apt-cache' nor 'dnf' was found in your PATH.");
        println!(
            "     -> Note: Self-update checks require a supported package manager (APT or DNF)."
        );
        Ok(())
    }
}
