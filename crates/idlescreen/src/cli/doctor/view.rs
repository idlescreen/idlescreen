// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Diagnostic view formatting for concise, verbose, and JSON outputs.

use super::inspect::DoctorReport;

pub fn print_help() {
    println!(
        "Usage: idlescreen doctor [options]\n\nOptions:\n  -v, --verbose, --all   Detailed itemized diagnostic breakdown\n  --json                 Machine-readable JSON diagnostic output\n  --fix                  Repair configuration, runtime permissions, and stale locks\n  -h, --help             Show this help message"
    );
}

pub fn print_concise(r: &DoctorReport) {
    let mark = |ok: bool, f: &'static str| if ok { "✓" } else { f };
    println!("=== IdleScreen System Diagnostics ===\n");
    if let Some(ref disp) = r.wayland_display {
        let de = r.desktop_env.as_deref().unwrap_or("Desktop");
        let dir_s = if r.runtime_dir_ok { "OK" } else { "missing" };
        println!("  [✓] Wayland Session : {disp} ({de}) [runtime dir {dir_s}]");
        let pm = mark(r.wayland_presenter_ok, "!");
        println!("  [{pm}] Presenter Sanity: {}", r.wayland_presenter_note);
    } else {
        println!("  [!] Wayland Session : not set (headless or X11 environment)");
        println!(
            "  [-] Presenter Sanity: headless ({}ms input debounce configured)",
            r.input_debounce_ms
        );
    }

    let env = r.runtime_env;
    println!("  [✓] Runtime Environment: {env}");

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

    println!(
        "  [✓] Active Saver    : {} (timeout: {}m, dwell: {:.1}s)",
        r.active_saver,
        r.timeout_secs / 60,
        r.dwell_secs
    );

    let tag = if r.config_is_toml {
        "TOML active"
    } else {
        "Legacy YAML"
    };
    let cfg_str = r.config_path.as_ref().map_or_else(
        || "Default built-in configuration".to_string(),
        |p| format!("{} ({tag})", p.display()),
    );
    println!("  [✓] Configuration   : {cfg_str}");
    println!("  [✓] Savers Engine   : built-in 16-bit arcade engine (52 scenes)");

    let shm_str = if r.shm_writable {
        "/dev/shm writable"
    } else {
        "/dev/shm unwritable"
    };
    let ipc_str = match (
        r.daemon_running && r.dbus_service_active,
        r.inhibited,
        r.session_bus_connected,
    ) {
        (true, true, _) => "D-Bus service connected, inhibited",
        (true, false, _) => "D-Bus service connected, uninhibited",
        (false, _, true) => "D-Bus session bus available",
        (false, _, false) => "D-Bus unavailable",
    };
    println!("  [✓] System & IPC    : {shm_str}, {ipc_str}\n");

    if r.wayland_display.is_some() && r.runtime_dir_ok && r.shm_writable && r.wayland_presenter_ok {
        println!("Diagnostics complete. All core systems nominal.");
    } else if r.wayland_display.is_some() && !r.wayland_presenter_ok {
        println!("Diagnostics complete. Issues detected with Wayland presentation.");
    } else {
        println!("Diagnostics complete.");
    }
}

pub fn print_verbose(r: &DoctorReport) {
    let mark = |ok: bool, f: &'static str| if ok { "✓" } else { f };
    println!("=== IdleScreen Detailed Diagnostics ===\n\nDisplay & Environment:");
    println!(
        "  [{}] WAYLAND_DISPLAY: {}",
        mark(r.wayland_display.is_some(), "!"),
        r.wayland_display.as_deref().unwrap_or("not set")
    );
    if let Some(ref de) = r.desktop_env {
        println!("  [✓] Desktop Environment: {de}");
    }
    if let Some(ref dir) = r.xdg_runtime_dir {
        let desc = if r.runtime_dir_ok {
            "accessible"
        } else {
            "invalid"
        };
        println!(
            "  [{}] XDG_RUNTIME_DIR: {dir} ({desc})",
            mark(r.runtime_dir_ok, "✗")
        );
    }
    let env = r.runtime_env;
    println!("  [✓] Runtime Environment: {env}");
    println!(
        "  [{}] Presenter Sanity: {}",
        mark(r.wayland_presenter_ok, "!"),
        r.wayland_presenter_note
    );
    println!(
        "  [✓] Input Debounce Window: {}ms (motion settling & key release ignore)",
        r.input_debounce_ms
    );

    println!("\nDaemon & Service:");
    let proc_str = if r.daemon_running {
        format!("PID {:?} (running)", r.daemon_pid.unwrap_or(0))
    } else {
        "not running".to_string()
    };
    println!(
        "  [{}] Process Status: {proc_str}",
        mark(r.daemon_running, "-")
    );

    if let Some(ref sock) = r.control_sock_path {
        let desc = if r.control_sock_active {
            "active"
        } else {
            "inactive"
        };
        println!(
            "  [{}] Control Socket: {} ({desc})",
            mark(r.control_sock_active, "-"),
            sock.display()
        );
    }

    let dbus_desc = if r.dbus_service_active {
        "connected (io.github.idlescreen.Idle)"
    } else {
        "inactive (io.github.idlescreen.Idle not registered)"
    };
    println!(
        "  [{}] D-Bus Service: {dbus_desc}",
        mark(r.dbus_service_active, "-")
    );
    println!(
        "  [{}] Systemd Service: {}",
        mark(r.systemd_active, "-"),
        r.systemd_desc
    );

    let inhib_desc = if !r.daemon_running {
        "inactive (daemon not running)"
    } else if r.inhibited {
        "inhibited (screensaver paused)"
    } else {
        "uninhibited (idle can trigger savers)"
    };
    println!(
        "  [{}] Inhibitor Status: {inhib_desc}",
        mark(r.daemon_running, "-")
    );

    println!("\nConfiguration & Savers:");
    let tag = if r.config_is_toml {
        "TOML active"
    } else {
        "YAML"
    };
    let cfg_str = r.config_path.as_ref().map_or_else(
        || "Default built-in configuration".to_string(),
        |p| format!("{} ({tag})", p.display()),
    );
    println!("  [✓] Config File: {cfg_str}");
    println!(
        "  [✓] Active Saver: {} (mode: {}, dwell: {:.1}s, timeout: {}s)",
        r.active_saver, r.mode_desc, r.dwell_secs, r.timeout_secs
    );
    println!(
        "  [✓] Built-in Engine: 16-bit arcade animation engine (52 scenes)\n\nPermissions & IPC:"
    );
    let (shm_m, shm_d) = if r.shm_writable {
        ("✓", "writable (wl_shm ready)")
    } else {
        ("✗", "not writable")
    };
    println!("  [{shm_m}] Shared Memory: /dev/shm {shm_d}");
    let (bus_m, bus_d) = if r.session_bus_connected {
        ("✓", "connected")
    } else {
        ("✗", "unavailable")
    };
    println!("  [{bus_m}] D-Bus Session: {bus_d}\n\nDetailed diagnostics complete.");
}

pub fn print_json(r: &DoctorReport) {
    let nominal =
        r.wayland_display.is_some() && r.shm_writable && r.runtime_dir_ok && r.wayland_presenter_ok;
    let disp = r.wayland_display.as_deref().unwrap_or("");
    let de = r.desktop_env.as_deref().unwrap_or("");
    let cfg = r
        .config_path
        .as_ref()
        .map_or_else(String::new, |p| p.display().to_string());
    let pid = r.daemon_pid.unwrap_or(0);
    println!(
        r#"{{"nominal":{nominal},"session":{{"wayland_display":"{disp}","desktop":"{de}","runtime_dir_ok":{},"runtime_env":"{}"}},"presenter":{{"ok":{},"debounce_ms":{},"note":"{}"}},"daemon":{{"running":{},"pid":{pid},"control_active":{},"dbus_service_active":{},"systemd_active":{},"inhibited":{}}},"config":{{"path":"{cfg}","is_toml":{},"active_saver":"{}","timeout_secs":{},"dwell_secs":{:.1}}},"savers":{{"builtin_ascii":true,"scenes":52}},"system":{{"shm_writable":{},"session_bus_connected":{}}}}}"#,
        r.runtime_dir_ok,
        r.runtime_env,
        r.wayland_presenter_ok,
        r.input_debounce_ms,
        r.wayland_presenter_note,
        r.daemon_running,
        r.control_sock_active,
        r.dbus_service_active,
        r.systemd_active,
        r.inhibited,
        r.config_is_toml,
        r.active_saver,
        r.timeout_secs,
        r.dwell_secs,
        r.shm_writable,
        r.session_bus_connected
    );
}
