// SPDX-License-Identifier: MIT

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use trance_dbus::TranceClient;

pub fn run_doctor() -> Result<(), String> {
    println!("==========================================");
    println!("Trance System Diagnostics (Doctor)");
    println!("==========================================");

    let mut failed = false;

    // 1. Wayland Session Check
    match std::env::var("WAYLAND_DISPLAY")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(display) => println!(" [✔] Wayland Session: WAYLAND_DISPLAY is set to '{display}'."),
        None => {
            println!(" [✗] Wayland Session: WAYLAND_DISPLAY environment variable is not set!");
            println!("     -> Fix: Trance daemon requires a running Wayland compositor.");
            failed = true;
        }
    }

    // 2. D-Bus Connectivity Check
    let dbus_ok = match TranceClient::connect() {
        Ok(_) => {
            println!(" [✔] D-Bus Connectivity: Connected to session service '{}'.", trance_dbus::SERVICE_NAME);
            true
        }
        Err(e) => {
            println!(" [✗] D-Bus Connectivity: Failed to connect to daemon: {e}");
            false
        }
    };

    // 3. Systemd User Service Check
    let systemd_status = Command::new("systemctl")
        .args(["--user", "is-active", "trance-daemon"])
        .output();
    match systemd_status {
        Ok(output) => {
            let active = String::from_utf8_lossy(&output.stdout).trim() == "active";
            if active {
                println!(" [✔] Systemd Service: trance-daemon.service is active.");
            } else if dbus_ok {
                println!(
                    " [!] Systemd Service: Daemon is active, but systemd service is not reported active."
                );
            } else {
                println!(" [✗] Systemd Service: trance-daemon.service is inactive or failed.");
                println!(
                    "     -> Fix: Start the service with: systemctl --user start trance-daemon"
                );
                failed = true;
            }
        }
        Err(_) => {
            println!(" [!] Systemd Service: 'systemctl' command not found or not usable.");
        }
    }

    // 4. Daemon PID Check
    let pid_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("trance-daemon.pid")
    } else {
        std::env::temp_dir().join("trance-daemon.pid")
    };
    if pid_path.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_path)
            && let Ok(pid) = pid_str.trim().parse::<i32>()
        {
            unsafe {
                if libc::kill(pid, 0) == 0 {
                    println!(" [✔] Process Status: Daemon is running (PID {pid}) and responsive.");
                } else {
                    println!(
                        " [✗] Process Status: Stale PID file exists (PID {pid}), but daemon is not running."
                    );
                    println!("     -> Fix: Clean up stale PID or restart the daemon.");
                    failed = true;
                }
            }
        }
    } else if dbus_ok {
        println!(" [!] Process Status: Connected to daemon via D-Bus, but PID file is missing.");
    } else {
        println!(" [✗] Process Status: Daemon PID file does not exist.");
    }

    // 5. Config File Check
    let config_path = get_config_path();
    match config_path {
        Some(path) => {
            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        println!(" [✔] Configuration: File found at '{}'.", path.display());
                        let line_count = content.lines().count();
                        println!(
                            "     -> Health check: Configuration file read successfully ({} lines).",
                            line_count
                        );
                    }
                    Err(e) => {
                        println!(
                            " [✗] Configuration: Found at '{}' but unreadable: {}",
                            path.display(),
                            e
                        );
                        failed = true;
                    }
                }
            } else {
                println!(" [!] Configuration: File not found. Default settings will be used.");
                println!(
                    "     -> Note: Config file path is expected at '{}'.",
                    path.display()
                );
            }
        }
        None => {
            println!(" [✗] Configuration: Could not resolve home directory path for settings.");
            failed = true;
        }
    }

    // 6. Monospace Fonts check
    if font_check_via_fc_list() {
        println!(" [✔] System Fonts: Monospace font is installed.");
    } else {
        println!(" [✗] System Fonts: Monospace font not found on system!");
        println!("     -> Fix: Please install fonts-dejavu-core or a system monospace font.");
        failed = true;
    }

    println!("==========================================");
    if failed {
        println!("Diagnostics complete: PROBLEMS DETECTED.");
        Err("Diagnostics check failed. Please resolve the issues marked with [✗].".to_string())
    } else {
        println!("Diagnostics complete: ALL SYSTEMS NOMINAL.");
        Ok(())
    }
}

fn get_config_path() -> Option<PathBuf> {
    let get_path_for_org = |org: &str| -> Option<PathBuf> {
        if let Some(xdg_config) = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|s| !s.is_empty())
        {
            return Some(PathBuf::from(xdg_config).join(org).join("theme.yaml"));
        }
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join(org)
                .join("theme.yaml"),
        )
    };

    let new_path = get_path_for_org("ubermetroid");
    if let Some(ref path) = new_path {
        if path.is_file() {
            return new_path;
        }
    }
    let legacy_path = get_path_for_org("local76");
    if let Some(ref path) = legacy_path {
        if path.is_file() {
            return legacy_path;
        }
    }
    new_path
}

fn font_check_via_fc_list() -> bool {
    let output = Command::new("fc-list").args([":mono"]).output();
    match output {
        Ok(out) => out.status.success() && !out.stdout.is_empty(),
        Err(_) => {
            let common_dirs = ["/usr/share/fonts", "/usr/local/share/fonts"];
            common_dirs.iter().any(|dir| PathBuf::from(dir).exists())
        }
    }
}
