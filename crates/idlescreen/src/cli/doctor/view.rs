// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Diagnostic view formatting for concise, verbose, and JSON outputs.

use super::inspect::DoctorReport;

pub fn print_help() {
    println!("Usage: idlescreen doctor [options]\n");
    println!("Options:");
    println!("  -v, --verbose, --all   Detailed itemized diagnostic breakdown");
    println!("  --json                 Machine-readable JSON diagnostic output");
    println!("  -h, --help             Show this help message");
}

pub fn print_concise(r: &DoctorReport) {
    println!("=== IdleScreen System Diagnostics ===\n");
    if let Some(ref disp) = r.wayland_display {
        let de = r.desktop_env.as_deref().unwrap_or("Desktop");
        let dir_status = if r.runtime_dir_ok { "OK" } else { "missing" };
        println!("  [✓] Wayland Session : {disp} ({de}) [runtime dir {dir_status}]");
    } else {
        println!("  [!] Wayland Session : not set (headless or X11 environment)");
    }

    if r.daemon_running {
        let mut parts = Vec::new();
        if let Some(pid) = r.daemon_pid {
            parts.push(format!("PID {pid}"));
        }
        if r.control_sock_active {
            parts.push("control socket connected".to_string());
        }
        if r.dbus_service_active {
            parts.push("D-Bus active".to_string());
        }
        let detail = if parts.is_empty() {
            "running".to_string()
        } else {
            parts.join(", ")
        };
        println!("  [✓] Daemon Status   : active ({detail})");
    } else {
        println!("  [-] Daemon Status   : inactive (not running)");
    }

    let mins = r.timeout_secs / 60;
    println!(
        "  [✓] Active Saver    : {} (timeout: {mins}m, dwell: {:.1}s)",
        r.active_saver, r.dwell_secs
    );

    let cfg_str = r.config_path.as_ref().map_or_else(
        || "Default built-in configuration".to_string(),
        |p| {
            let tag = if r.config_is_toml {
                "TOML active"
            } else {
                "Legacy YAML"
            };
            format!("{} ({tag})", p.display())
        },
    );
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
    let ipc_str = if r.daemon_running && r.dbus_service_active {
        if r.inhibited {
            "D-Bus service connected, inhibited"
        } else {
            "D-Bus service connected, uninhibited"
        }
    } else if r.session_bus_connected {
        "D-Bus session bus available"
    } else {
        "D-Bus unavailable"
    };
    println!("  [✓] System & IPC    : {shm_str}, {ipc_str}\n");

    if r.wayland_display.is_some() && r.runtime_dir_ok && r.shm_writable {
        println!("Diagnostics complete. All core systems nominal.");
    } else {
        println!("Diagnostics complete.");
    }
}

pub fn print_verbose(r: &DoctorReport) {
    println!("=== IdleScreen Detailed Diagnostics ===\n");
    println!("Display & Environment:");
    let disp = r.wayland_display.as_deref().unwrap_or("not set");
    let disp_mark = if r.wayland_display.is_some() { "✓" } else { "!" };
    println!("  [{disp_mark}] WAYLAND_DISPLAY: {disp}");
    if let Some(ref de) = r.desktop_env {
        println!("  [✓] Desktop Environment: {de}");
    }
    if let Some(ref dir) = r.xdg_runtime_dir {
        let dir_mark = if r.runtime_dir_ok { "✓" } else { "✗" };
        let dir_desc = if r.runtime_dir_ok {
            "accessible"
        } else {
            "invalid"
        };
        println!("  [{dir_mark}] XDG_RUNTIME_DIR: {dir} ({dir_desc})");
    }

    println!("\nDaemon & Service:");
    let proc_mark = if r.daemon_running { "✓" } else { "-" };
    let proc_str = if r.daemon_running {
        format!("PID {:?} (running)", r.daemon_pid.unwrap_or(0))
    } else {
        "not running".to_string()
    };
    println!("  [{proc_mark}] Process Status: {proc_str}");

    if let Some(ref sock) = r.control_sock_path {
        let sock_mark = if r.control_sock_active { "✓" } else { "-" };
        let sock_desc = if r.control_sock_active {
            "active"
        } else {
            "inactive"
        };
        println!(
            "  [{sock_mark}] Control Socket: {} ({sock_desc})",
            sock.display()
        );
    }

    let dbus_mark = if r.dbus_service_active { "✓" } else { "-" };
    let dbus_desc = if r.dbus_service_active {
        "connected (io.github.idlescreen.Idle)"
    } else {
        "inactive (io.github.idlescreen.Idle not registered)"
    };
    println!("  [{dbus_mark}] D-Bus Service: {dbus_desc}");

    let sysd_mark = if r.systemd_active { "✓" } else { "-" };
    println!("  [{sysd_mark}] Systemd Service: {}", r.systemd_desc);

    let inhib_mark = if r.daemon_running { "✓" } else { "-" };
    let inhib_desc = if !r.daemon_running {
        "inactive (daemon not running)"
    } else if r.inhibited {
        "inhibited (screensaver paused)"
    } else {
        "uninhibited (idle can trigger savers)"
    };
    println!("  [{inhib_mark}] Inhibitor Status: {inhib_desc}");

    println!("\nConfiguration & Savers:");
    let cfg_str = r.config_path.as_ref().map_or_else(
        || "Default built-in configuration".to_string(),
        |p| {
            format!(
                "{} ({})",
                p.display(),
                if r.config_is_toml { "TOML active" } else { "YAML" }
            )
        },
    );
    println!("  [✓] Config File: {cfg_str}");
    println!(
        "  [✓] Active Saver: {} (mode: {}, dwell: {:.1}s, timeout: {}s)",
        r.active_saver, r.mode_desc, r.dwell_secs, r.timeout_secs
    );
    println!("  [✓] Built-in: ASCII animation engine (37 effects)");
    let ext_str = r.extras_bin.as_ref().map_or_else(
        || "not installed (built-in ASCII available)".to_string(),
        |e| format!("{} (11 math savers)", e.display()),
    );
    let ext_mark = if r.extras_bin.is_some() { "✓" } else { "-" };
    println!("  [{ext_mark}] Extras Helper: {ext_str}");

    println!("\nPermissions & IPC:");
    let shm_mark = if r.shm_writable { "✓" } else { "✗" };
    let shm_desc = if r.shm_writable {
        "writable (wl_shm ready)"
    } else {
        "not writable"
    };
    println!("  [{shm_mark}] Shared Memory: /dev/shm {shm_desc}");

    let bus_mark = if r.session_bus_connected { "✓" } else { "✗" };
    let bus_desc = if r.session_bus_connected {
        "connected"
    } else {
        "unavailable"
    };
    println!("  [{bus_mark}] D-Bus Session: {bus_desc}");

    println!("\nDetailed diagnostics complete.");
}

pub fn print_json(r: &DoctorReport) {
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
        r#"{{"nominal":{},"session":{{"wayland_display":"{}","desktop":"{}","runtime_dir_ok":{}}},"daemon":{{"running":{},"pid":{},"control_active":{},"dbus_service_active":{},"systemd_active":{},"inhibited":{}}},"config":{{"path":"{}","is_toml":{},"active_saver":"{}","timeout_secs":{},"dwell_secs":{:.1}}},"savers":{{"builtin_ascii":true,"extras_helper":"{}"}},"system":{{"shm_writable":{},"session_bus_connected":{}}}}}"#,
        nominal,
        disp,
        de,
        r.runtime_dir_ok,
        r.daemon_running,
        r.daemon_pid.unwrap_or(0),
        r.control_sock_active,
        r.dbus_service_active,
        r.systemd_active,
        r.inhibited,
        cfg,
        r.config_is_toml,
        r.active_saver,
        r.timeout_secs,
        r.dwell_secs,
        ext,
        r.shm_writable,
        r.session_bus_connected
    );
}
