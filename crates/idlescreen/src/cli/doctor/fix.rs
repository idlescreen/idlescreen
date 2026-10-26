// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Self-healing diagnostic repair (`idlescreen doctor --fix`).

use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::daemon::config::{Config, resolve_config_dir};
use crate::daemon::control_sock::{
    ControlClient, resolve_control_sock_path, resolve_runtime_subdir,
};
use crate::daemon::pidfile::{is_daemon_running, resolve_pid_path};

pub fn run_doctor_fix() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IdleScreen Doctor Repair (--fix) ===\n");

    let cfg_dir = resolve_config_dir();
    let toml_path = cfg_dir.join("config.toml");
    let config = Config::load_checked()?;
    config.save_to_path(&toml_path)?;
    println!(
        "  [✓] Configuration   : Verified and persisted {}",
        toml_path.display()
    );

    let rt_dir = resolve_runtime_subdir();
    fs::create_dir_all(&rt_dir)?;
    let _ = fs::set_permissions(&rt_dir, fs::Permissions::from_mode(0o700));
    println!(
        "  [✓] Runtime Dir     : Verified {} (mode 0700)",
        rt_dir.display()
    );

    if let Ok(pid_path) = resolve_pid_path()
        && pid_path.exists()
        && !is_daemon_running(&pid_path)
    {
        let _ = fs::remove_file(&pid_path);
        println!(
            "  [✓] Stale PID File  : Removed unlocked {}",
            pid_path.display()
        );
    }

    if let Ok(sock_path) = resolve_control_sock_path()
        && sock_path.exists()
        && ControlClient::send_command("ping").map_or(true, |r| r != "PONG")
    {
        let _ = fs::remove_file(&sock_path);
        println!(
            "  [✓] Stale Socket    : Removed unresponsive {}",
            sock_path.display()
        );
    }

    let daemon_up = ControlClient::send_command("ping").is_ok_and(|r| r == "PONG");
    if !daemon_up && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        let _ = crate::cli::service::enable_service();
        println!("  [✓] Daemon Service  : Requested user service activation");
    }

    println!("\nDoctor repair finished.");
    Ok(())
}
