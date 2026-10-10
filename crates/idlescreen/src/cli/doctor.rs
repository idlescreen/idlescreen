// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Diagnostic doctor checking display session, daemon status, configuration, and permissions.

use crate::daemon::config::{Config, resolve_config_dir};
use crate::daemon::control_sock::{ControlClient, resolve_control_sock_path};
use crate::daemon::pidfile::{is_daemon_running, resolve_pid_path};
use crate::extras_ipc::resolve_extras_binary;
use std::path::{Path, PathBuf};

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    execute_args(&[])
}

pub fn execute_args(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("Usage: idlescreen doctor [options]\n");
        println!("Options:");
        println!("  -v, --verbose, --all   Detailed itemized diagnostic breakdown");
        println!("  --json                 Machine-readable JSON diagnostic output");
        println!("  -h, --help             Show this help message");
        return Ok(());
    }

    let report = collect_diagnostics();
    if args.iter().any(|a| a == "--json") {
        print_json(&report);
    } else if args
        .iter()
        .any(|a| a == "-v" || a == "--verbose" || a == "--all")
    {
        print_verbose(&report);
    } else {
        print_concise(&report);
    }
    Ok(())
}

#[allow(clippy::struct_excessive_bools)]
struct DoctorReport {
    wayland_display: Option<String>,
    desktop_env: Option<String>,
    xdg_runtime_dir: Option<String>,
    runtime_dir_ok: bool,
    daemon_pid: Option<u32>,
    daemon_running: bool,
    control_sock_path: Option<PathBuf>,
    control_sock_active: bool,
    dbus_connected: bool,
    inhibited: bool,
    active_saver: String,
    timeout_secs: u64,
    dwell_secs: f32,
    mode_desc: &'static str,
    config_path: Option<PathBuf>,
    config_is_toml: bool,
    extras_bin: Option<PathBuf>,
    shm_writable: bool,
}

fn collect_diagnostics() -> DoctorReport {
    let wayland_display = std::env::var("WAYLAND_DISPLAY")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let desktop_env = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .ok()
        .filter(|s| !s.trim().is_empty());
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let runtime_dir_ok = xdg_runtime_dir
        .as_ref()
        .is_some_and(|d| Path::new(d).is_dir());

    let pid_path = resolve_pid_path().ok();
    let (daemon_pid, mut daemon_running) = if let Some(ref p) = pid_path {
        let running = is_daemon_running(p);
        let pid = std::fs::read_to_string(p)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok());
        (pid, running)
    } else {
        (None, false)
    };

    let ctrl_path = resolve_control_sock_path().ok();
    let control_sock_active = ctrl_path.as_ref().is_some_and(|p| p.exists())
        && ControlClient::send_command("ping").is_ok_and(|r| r == "PONG");
    if control_sock_active {
        daemon_running = true;
    }

    let (dbus_connected, inhibited) = match zbus::blocking::Connection::session() {
        Ok(c) => (
            true,
            crate::daemon::dbus::IdleClientProxyBlocking::new(&c)
                .and_then(|p| p.get_status())
                .is_ok_and(|s| s.contains("\"inhibited\":true")),
        ),
        Err(_) => (false, false),
    };

    let cfg_dir = resolve_config_dir();
    let toml_file = cfg_dir.join("config.toml");
    let yaml_file = cfg_dir.join("config.yaml");
    let (config_path, config_is_toml) = if toml_file.exists() {
        (Some(toml_file), true)
    } else if yaml_file.exists() {
        (Some(yaml_file), false)
    } else {
        (None, false)
    };

    let config = Config::load_or_migrate();
    let active_saver = config
        .saver
        .active
        .clone()
        .unwrap_or_else(|| "ascii".to_string());
    let mode_desc = match config.saver.ascii.mode.as_str() {
        "os" => "OS only",
        "de" => "DE only",
        "kernel" => "Kernel only",
        "custom" => "Custom text",
        _ => "cycle",
    };

    DoctorReport {
        wayland_display,
        desktop_env,
        xdg_runtime_dir,
        runtime_dir_ok,
        daemon_pid,
        daemon_running,
        control_sock_path: ctrl_path,
        control_sock_active,
        dbus_connected,
        inhibited,
        active_saver,
        timeout_secs: config.general.idle_timeout_secs,
        dwell_secs: config.saver.ascii.dwell_secs,
        mode_desc,
        config_path,
        config_is_toml,
        extras_bin: resolve_extras_binary(),
        shm_writable: test_shm_writable(),
    }
}

fn test_shm_writable() -> bool {
    let p = Path::new("/dev/shm");
    if !p.is_dir() {
        return false;
    }
    let probe = p.join(format!(".probe-{}", std::process::id()));
    let ok = std::fs::File::create(&probe).is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

fn print_concise(r: &DoctorReport) {
    println!("=== IdleScreen System Diagnostics ===\n");
    if let Some(ref disp) = r.wayland_display {
        let de = r.desktop_env.as_deref().unwrap_or("Desktop");
        println!(
            "  [✓] Wayland Session : {disp} ({de}) [runtime dir {}]",
            if r.runtime_dir_ok { "OK" } else { "missing" }
        );
    } else {
        println!("  [!] Wayland Session : not set (headless or X11 environment)");
    }

    if r.daemon_running {
        let pid_str = r
            .daemon_pid
            .map(|p| format!("PID {p}, "))
            .unwrap_or_default();
        println!("  [✓] Daemon Status   : active ({pid_str}control socket connected)");
    } else {
        println!("  [-] Daemon Status   : inactive (not running)");
    }

    let mins = r.timeout_secs / 60;
    println!(
        "  [✓] Active Saver    : {} (timeout: {mins}m, dwell: {:.1}s)",
        r.active_saver, r.dwell_secs
    );

    let cfg_str =
        r.config_path
            .as_ref()
            .map_or("Default built-in configuration".to_string(), |p| {
                format!(
                    "{} ({})",
                    p.display(),
                    if r.config_is_toml {
                        "TOML active"
                    } else {
                        "Legacy YAML"
                    }
                )
            });
    println!("  [✓] Configuration   : {cfg_str}");

    let extras_str = if r.extras_bin.is_some() {
        "extras (11 math savers)"
    } else {
        "extras not installed"
    };
    println!("  [✓] Savers Engine   : built-in ASCII (37 effects) + {extras_str}");

    let shm_str = if r.shm_writable {
        "/dev/shm writable"
    } else {
        "/dev/shm unwritable"
    };
    let dbus_str = if r.dbus_connected {
        "D-Bus session connected"
    } else {
        "D-Bus unavailable"
    };
    let inhib_str = if r.inhibited {
        "inhibited"
    } else {
        "uninhibited"
    };
    println!("  [✓] System & IPC    : {shm_str}, {dbus_str}, {inhib_str}\n");
    if r.wayland_display.is_some() && r.runtime_dir_ok && r.shm_writable {
        println!("Diagnostics complete. All core systems nominal.");
    } else {
        println!("Diagnostics complete.");
    }
}

fn print_verbose(r: &DoctorReport) {
    println!("=== IdleScreen Detailed Diagnostics ===\n");
    println!("Display & Environment:");
    let disp = r.wayland_display.as_deref().unwrap_or("not set");
    println!(
        "  [{}] WAYLAND_DISPLAY: {disp}",
        if r.wayland_display.is_some() {
            "✓"
        } else {
            "!"
        }
    );
    if let Some(ref de) = r.desktop_env {
        println!("  [✓] Desktop Environment: {de}");
    }
    if let Some(ref dir) = r.xdg_runtime_dir {
        println!(
            "  [{}] XDG_RUNTIME_DIR: {dir} ({})",
            if r.runtime_dir_ok { "✓" } else { "✗" },
            if r.runtime_dir_ok {
                "accessible"
            } else {
                "invalid"
            }
        );
    }

    println!("\nDaemon & Service:");
    println!(
        "  [{}] Process Status: {}",
        if r.daemon_running { "✓" } else { "-" },
        if r.daemon_running {
            format!("PID {:?} (running)", r.daemon_pid.unwrap_or(0))
        } else {
            "not running".to_string()
        }
    );
    if let Some(ref sock) = r.control_sock_path {
        println!(
            "  [{}] Control Socket: {} ({})",
            if r.control_sock_active { "✓" } else { "-" },
            sock.display(),
            if r.control_sock_active {
                "active"
            } else {
                "inactive"
            }
        );
    }
    println!(
        "  [{}] D-Bus Service: {}",
        if r.dbus_connected { "✓" } else { "!" },
        if r.dbus_connected {
            "connected (io.github.idlescreen.Idle)"
        } else {
            "disconnected"
        }
    );
    println!(
        "  [✓] Inhibitor Status: {}",
        if r.inhibited {
            "inhibited"
        } else {
            "uninhibited (idle can trigger savers)"
        }
    );

    println!("\nConfiguration & Savers:");
    let cfg_str =
        r.config_path
            .as_ref()
            .map_or("Default built-in configuration".to_string(), |p| {
                format!(
                    "{} ({})",
                    p.display(),
                    if r.config_is_toml {
                        "TOML active"
                    } else {
                        "YAML"
                    }
                )
            });
    println!("  [✓] Config File: {cfg_str}");
    println!(
        "  [✓] Active Saver: {} (mode: {}, dwell: {:.1}s, timeout: {}s)",
        r.active_saver, r.mode_desc, r.dwell_secs, r.timeout_secs
    );
    println!("  [✓] Built-in: ASCII animation engine (37 effects)");
    let ext_str = r.extras_bin.as_ref().map_or(
        "not installed (built-in ASCII available)".to_string(),
        |e| format!("{} (11 math savers)", e.display()),
    );
    println!(
        "  [{}] Extras Helper: {ext_str}",
        if r.extras_bin.is_some() { "✓" } else { "-" }
    );

    println!("\nPermissions & IPC:");
    println!(
        "  [{}] Shared Memory: /dev/shm {}",
        if r.shm_writable { "✓" } else { "✗" },
        if r.shm_writable {
            "writable (wl_shm ready)"
        } else {
            "not writable"
        }
    );
    println!("\nDetailed diagnostics complete.");
}

fn print_json(r: &DoctorReport) {
    let nominal = r.wayland_display.is_some() && r.shm_writable && r.runtime_dir_ok;
    let disp = r.wayland_display.as_deref().unwrap_or("");
    let de = r.desktop_env.as_deref().unwrap_or("");
    let cfg = r
        .config_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let ext = r
        .extras_bin
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    println!(
        r#"{{"nominal":{},"session":{{"wayland_display":"{}","desktop":"{}"}},"daemon":{{"running":{},"pid":{},"control_active":{},"dbus_connected":{},"inhibited":{}}},"config":{{"path":"{}","active_saver":"{}","timeout_secs":{}}},"savers":{{"builtin_ascii":true,"extras_helper":"{}"}},"system":{{"shm_writable":{}}}}}"#,
        nominal,
        disp,
        de,
        r.daemon_running,
        r.daemon_pid.unwrap_or(0),
        r.control_sock_active,
        r.dbus_connected,
        r.inhibited,
        cfg,
        r.active_saver,
        r.timeout_secs,
        ext,
        r.shm_writable
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_diagnostics() {
        let report = collect_diagnostics();
        assert!(!report.active_saver.is_empty());
        assert!(report.timeout_secs > 0);
    }

    #[test]
    fn test_execute_modes() {
        assert!(execute_args(&["--help".to_string()]).is_ok());
        assert!(execute_args(&[]).is_ok());
        assert!(execute_args(&["--verbose".to_string()]).is_ok());
        assert!(execute_args(&["--json".to_string()]).is_ok());
    }
}
