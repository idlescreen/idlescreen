// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! D-Bus / CLI / systemd helpers for talking to `idlescreen` from the panel applet.

#![allow(unsafe_code)]

use std::process::Command;
use std::time::Duration;

use crate::config::ThemeConfig;
use crate::dbus_bridge::{
    Context, DaemonStatus, Result, daemon_available, detect_screensavers, err_msg, run_idlescreen,
    start_service, stop_service,
};

pub fn is_running() -> bool {
    daemon_available()
}

/// Start the user unit and enable it so it returns after login/upgrades.
pub fn start_daemon_service() -> Result<()> {
    start_service().context("could not start idlescreen.service")
}

/// Stop the running user unit (does **not** disable — keeps login autostart).
pub fn stop_daemon_service() -> Result<()> {
    stop_service().context("could not stop idlescreen.service")
}

pub fn fetch_status() -> Result<DaemonStatus> {
    let cfg = ThemeConfig::load();
    let mut status = DaemonStatus {
        idle_enabled: cfg.idle_enabled,
        idle_timeout_mins: cfg.idle_timeout_mins,
        active_saver: cfg.active_saver.unwrap_or_else(|| "ascii".to_string()),
        show_fps_overlay: cfg.show_fps_overlay,
        render_scale: cfg
            .render_scale
            .map(|s| s.to_string())
            .unwrap_or_else(|| "null".to_string()),
    };
    if let Ok(json) = run_idlescreen(&["status", "--json"])
        && let Some(idx) = json.find("\"active_saver\":\"")
    {
        let rest = &json[idx + 16..];
        if let Some(end) = rest.find('"') {
            status.active_saver = rest[..end].to_string();
        }
    }
    Ok(status)
}

pub fn set_idle_enabled(enabled: bool) -> Result<()> {
    let mut cfg = ThemeConfig::load();
    cfg.idle_enabled = enabled;
    cfg.save_field("idle_enabled")
        .context("failed to save idle_enabled")
}

pub fn set_timeout(minutes: u32) -> Result<()> {
    let mins_str = minutes.to_string();
    run_idlescreen(&["timeout", &mins_str])
        .map(|_| ())
        .context("failed to set idle timeout")
}

pub fn set_active_saver(name: Option<&str>) -> Result<()> {
    let target = name.unwrap_or("ascii");
    run_idlescreen(&["set", target])
        .map(|_| ())
        .context("failed to set active screensaver")
}

pub fn set_show_fps_overlay(enabled: bool) -> Result<()> {
    let mut cfg = ThemeConfig::load();
    cfg.show_fps_overlay = enabled;
    cfg.save_field("show_fps_overlay")
        .context("failed to set FPS overlay")
}

pub fn list_savers() -> Result<Vec<String>> {
    Ok(detect_screensavers())
}

pub fn set_render_scale(scale: f32) -> Result<()> {
    let mut cfg = ThemeConfig::load();
    cfg.render_scale = Some(scale);
    cfg.save_field("render_scale")
        .context("failed to set render scale")
}

/// Preview a saver: prefer `idlescreen start` or `idlescreen preview <name>`.
pub fn preview_saver(name: &str) -> Result<()> {
    if !is_running() {
        let _ = start_daemon_service();
    }
    let _ = set_active_saver(Some(name));
    if run_idlescreen(&["start"]).is_ok() {
        return Ok(());
    }
    if let Ok(mut child) = Command::new("idlescreen").args(["preview", name]).spawn() {
        std::thread::sleep(Duration::from_millis(200));
        if let Ok(Some(status)) = child.try_wait()
            && !status.success()
        {
            return Err(err_msg(format!(
                "preview process exited early with status: {status}"
            )));
        }
        return Ok(());
    }
    Err(err_msg("could not launch idlescreen preview"))
}

/// Launch the IdleScreen TUI in a terminal (requires a real TTY).
pub fn open_tui_dashboard() -> bool {
    let programs: &[&[&str]] = &[
        &["idlescreen", "tui"],
        &["idlescreen-tui"],
        &["idle-tui"],
        &["idle", "tui"],
    ];
    let terminals: &[(&str, &[&str])] = &[
        ("cosmic-term", &["-e"]),
        ("cosmic-term", &["--"]),
        ("kgx", &["-e"]),
        ("gnome-terminal", &["--"]),
        ("alacritty", &["-e"]),
        ("kitty", &[]),
        ("foot", &["-e"]),
        ("xterm", &["-e"]),
    ];

    for (term, term_flags) in terminals {
        if !Command::new("which")
            .arg(term)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            continue;
        }
        for prog in programs {
            let mut cmd = Command::new(term);
            cmd.args(*term_flags);
            for p in *prog {
                cmd.arg(p);
            }
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                // SAFETY: `setsid()` is async-signal-safe in POSIX and detaches the
                // spawned terminal session before `exec` without mutating Rust state.
                unsafe {
                    cmd.pre_exec(|| {
                        libc::setsid();
                        Ok(())
                    });
                }
            }
            if cmd.spawn().is_ok() {
                return true;
            }
        }
    }

    for desktop_id in ["io.github.idlescreen.tui", "idlescreen"] {
        if Command::new("gtk-launch").arg(desktop_id).spawn().is_ok() {
            return true;
        }
    }
    false
}
