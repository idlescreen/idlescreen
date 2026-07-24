// SPDX-License-Identifier: MIT

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;

#[tracing::instrument]
pub fn handle_bug_report() -> Result<()> {
    println!("Generating sanitized diagnostic report...");

    let mut report = String::new();
    report.push_str("### Trance Diagnostics & Bug Report\n\n");

    // System Environment
    report.push_str("#### Environment Settings\n");
    let display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "not set".to_string());
    report.push_str(&format!("* **WAYLAND_DISPLAY**: `{display}`\n"));
    let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "not set".to_string());
    report.push_str(&format!("* **XDG_RUNTIME_DIR**: `{xdg_runtime}`\n"));
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    report.push_str(&format!(
        "* **CLI version**: `{}`\n",
        env!("CARGO_PKG_VERSION")
    ));
    if let Some(pkg) = package_version_line() {
        report.push_str(&format!("* **Package**: `{pkg}`\n"));
    }
    report.push('\n');

    // Daemon & Service Status
    report.push_str("#### Service Status\n");
    let active_status = Command::new("systemctl")
        .args(["--user", "is-active", "idle-daemon"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    report.push_str(&format!("* **Systemd User Service**: `{active_status}`\n"));

    // Config Check
    report.push_str("\n#### Configuration Settings\n");
    let config_path = get_config_path();
    if let Some(ref path) = config_path {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                report.push_str("```yaml\n");
                let sanitized = content.replace(&home, "~");
                report.push_str(&sanitized);
                report.push_str("\n```\n");
            } else {
                report.push_str("*Config file exists but is unreadable.*\n");
            }
        } else {
            report.push_str("*No custom config file found. Using default values.*\n");
        }
    }

    // Daemon Logs (last 20 lines)
    report.push_str("\n#### Daemon Log Output (systemd journalctl)\n");
    let log_output = Command::new("journalctl")
        .args(["--user", "-u", "idle-daemon", "-n", "20", "--no-pager"])
        .output();
    match log_output {
        Ok(o) => {
            let log_str = String::from_utf8_lossy(&o.stdout);
            if log_str.trim().is_empty() {
                report.push_str("*Journal log is empty.*\n");
            } else {
                report.push_str("```text\n");
                let sanitized_logs = log_str.replace(&home, "~");
                report.push_str(&sanitized_logs);
                report.push_str("\n```\n");
            }
        }
        Err(_) => {
            report.push_str("*Could not retrieve journal logs.*\n");
        }
    }

    println!("\n==========================================");
    println!("Please copy the block below for your bug report:");
    println!("==========================================\n");
    println!("{report}");

    Ok(())
}

fn package_version_line() -> Option<String> {
    if let Ok(o) = Command::new("rpm")
        .args([
            "-q",
            "trance",
            "--qf",
            "%{NAME}-%{VERSION}-%{RELEASE}.%{ARCH}",
        ])
        .output()
        && o.status.success()
    {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(format!("rpm {s}"));
        }
    }
    if let Ok(o) = Command::new("dpkg-query")
        .args(["-W", "-f=${Package} ${Version}", "trance"])
        .output()
        && o.status.success()
    {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(format!("deb {s}"));
        }
    }
    None
}

fn get_config_path() -> Option<PathBuf> {
    if let Some(xdg_config) = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
    {
        return Some(PathBuf::from(xdg_config).join("idle").join("config.yaml"));
    }
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("idle")
            .join("config.yaml"),
    )
}
