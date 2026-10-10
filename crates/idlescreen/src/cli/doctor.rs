// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Diagnostic doctor checking environment, display sockets, permissions, and extras.

use std::path::Path;
use crate::daemon::config::resolve_config_dir;
use crate::daemon::control_sock::resolve_control_sock_path;
use crate::daemon::pidfile::resolve_pid_path;
use crate::extras_ipc::resolve_extras_binary;

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IdleScreen Doctor Diagnostics ===");
    println!();

    // 1. Wayland display
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(disp) if !disp.trim().is_empty() => {
            println!("  [✓] WAYLAND_DISPLAY: {disp}");
        }
        _ => {
            println!("  [!] WAYLAND_DISPLAY: not set (headless or X11 environment)");
        }
    }

    // 2. XDG_RUNTIME_DIR
    match std::env::var("XDG_RUNTIME_DIR") {
        Ok(dir) if !dir.trim().is_empty() => {
            let p = Path::new(&dir);
            if p.is_dir() {
                println!("  [✓] XDG_RUNTIME_DIR: {dir} (accessible)");
            } else {
                println!("  [✗] XDG_RUNTIME_DIR: {dir} (not a valid directory)");
            }
        }
        _ => {
            println!("  [✗] XDG_RUNTIME_DIR: not set");
        }
    }

    // 3. Configuration file
    let cfg_dir = resolve_config_dir();
    let toml_file = cfg_dir.join("config.toml");
    let yaml_file = cfg_dir.join("config.yaml");
    if toml_file.exists() {
        println!("  [✓] Config: {} (TOML active)", toml_file.display());
    } else if yaml_file.exists() {
        println!("  [!] Config: {} (Legacy YAML pending migration)", yaml_file.display());
    } else {
        println!("  [✓] Config: Default built-in configuration");
    }

    // 4. Daemon runtime status
    let pid_path = resolve_pid_path()?;
    if pid_path.exists() {
        if let Ok(pid) = std::fs::read_to_string(&pid_path) {
            println!("  [✓] Daemon PID: {}", pid.trim());
        }
    } else {
        println!("  [-] Daemon PID: not running");
    }

    // 5. Control socket
    let ctrl_path = resolve_control_sock_path()?;
    if ctrl_path.exists() {
        println!("  [✓] Control Socket: {}", ctrl_path.display());
    } else {
        println!("  [-] Control Socket: not active");
    }

    // 6. idlescreen-extras helper
    if let Some(extras_bin) = resolve_extras_binary() {
        println!("  [✓] Extras Helper: {}", extras_bin.display());
    } else {
        println!("  [-] Extras Helper: not installed (only built-in ASCII available)");
    }

    println!();
    println!("Diagnostics complete.");
    Ok(())
}
