// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Diagnostic inspection probes for display, daemon, D-Bus, and system health.

use crate::daemon::config::{Config, resolve_config_dir};
use crate::daemon::control_sock::{ControlClient, resolve_control_sock_path};
use crate::daemon::pidfile::{is_daemon_running, resolve_pid_path};
use crate::extras_ipc::resolve_extras_binary;
use std::path::{Path, PathBuf};

#[allow(clippy::struct_excessive_bools)]
pub struct DoctorReport {
    pub wayland_display: Option<String>,
    pub desktop_env: Option<String>,
    pub xdg_runtime_dir: Option<String>,
    pub runtime_dir_ok: bool,
    pub daemon_pid: Option<u32>,
    pub daemon_running: bool,
    pub control_sock_path: Option<PathBuf>,
    pub control_sock_active: bool,
    pub session_bus_connected: bool,
    pub dbus_service_active: bool,
    pub systemd_active: bool,
    pub systemd_desc: String,
    pub inhibited: bool,
    pub active_saver: String,
    pub timeout_secs: u64,
    pub dwell_secs: f32,
    pub mode_desc: &'static str,
    pub config_path: Option<PathBuf>,
    pub config_is_toml: bool,
    pub extras_bin: Option<PathBuf>,
    pub shm_writable: bool,
}

pub fn collect_diagnostics() -> DoctorReport {
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

    let (session_bus_connected, dbus_service_active, daemon_inhibited) =
        match zbus::blocking::Connection::session() {
            Ok(conn) => {
                if let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
                    && let Ok(status) = proxy.get_status()
                {
                    let inhibited = status.contains("\"inhibited\":true");
                    (true, true, inhibited)
                } else {
                    (true, false, false)
                }
            }
            Err(_) => (false, false, false),
        };

    if control_sock_active || dbus_service_active {
        daemon_running = true;
    }

    let (systemd_active, systemd_desc) = check_systemd_status();

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
        session_bus_connected,
        dbus_service_active,
        systemd_active,
        systemd_desc,
        inhibited: daemon_inhibited,
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

pub(crate) fn test_shm_writable() -> bool {
    let p = Path::new("/dev/shm");
    if !p.is_dir() {
        return false;
    }
    let probe = p.join(format!(".probe-{}", std::process::id()));
    let ok = std::fs::File::create(&probe).is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

pub(crate) fn check_systemd_status() -> (bool, String) {
    for unit in ["idlescreen.service", "idle-daemon.service"] {
        if let Ok(output) = std::process::Command::new("systemctl")
            .args(["--user", "is-active", unit])
            .output()
        {
            let active_state = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if active_state == "active" {
                let enabled_state = std::process::Command::new("systemctl")
                    .args(["--user", "is-enabled", unit])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();
                let desc = if enabled_state.is_empty() {
                    format!("{unit} (active)")
                } else {
                    format!("{unit} (active + {enabled_state})")
                };
                return (true, desc);
            }
        }
    }
    (false, "inactive".to_string())
}
