// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! D-Bus / systemd helpers for talking to `idle-daemon` from the panel applet.
//! Unit management lives in `idle_dbus::service` (shared with cli/tui).

use std::process::Command;
use std::time::Duration;

use idle_dbus::{DaemonStatus, TranceClient, daemon_available, service};
use idle_err::{Context, Result, bail};

pub fn is_running() -> bool {
    daemon_available()
}

/// Start the user unit and enable it so it returns after login/upgrades.
pub fn start_daemon_service() -> Result<()> {
    service::start_daemon_service().context("could not start idle-daemon")
}

/// Stop the running user unit (does **not** disable — keeps login autostart).
pub fn stop_daemon_service() -> Result<()> {
    service::stop_daemon_service().context("could not stop idle-daemon")
}

pub fn fetch_status() -> Result<DaemonStatus> {
    let client = TranceClient::connect().context("failed to connect to idle daemon")?;
    client.get_status().context("failed to fetch daemon status")
}

pub fn set_idle_enabled(enabled: bool) -> Result<()> {
    let client = TranceClient::connect().context("failed to connect to idle daemon")?;
    if enabled {
        client.enable().context("failed to enable idle activation")
    } else {
        client
            .disable()
            .context("failed to disable idle activation")
    }
}

pub fn set_timeout(minutes: u32) -> Result<()> {
    TranceClient::connect()
        .context("failed to connect to idle daemon")?
        .set_timeout(minutes)
        .context("failed to set idle timeout")
}

pub fn set_active_saver(name: Option<&str>) -> Result<()> {
    TranceClient::connect()
        .context("failed to connect to idle daemon")?
        .set_saver(name.unwrap_or(""))
        .context("failed to set active screensaver")
}

pub fn set_show_fps_overlay(enabled: bool) -> Result<()> {
    TranceClient::connect()
        .context("failed to connect to idle daemon")?
        .set_show_fps_overlay(enabled)
        .context("failed to set FPS overlay")
}

pub fn list_savers() -> Result<Vec<String>> {
    TranceClient::connect()
        .context("failed to connect to idle daemon")?
        .list_savers()
        .context("failed to list installed screensavers")
}

pub fn set_render_scale(scale: f32) -> Result<()> {
    TranceClient::connect()
        .context("failed to connect to idle daemon")?
        .set_render_scale(scale)
        .context("failed to set render scale")
}

/// Preview a saver: prefer the daemon D-Bus path (layer-shell overlay).
///
/// If the daemon is down, try to start it first. As a last resort, run the
/// packaged `idle-daemon run-plugin <name>` fullscreen helper.
pub fn preview_saver(name: &str) -> Result<()> {
    if !is_running() {
        idle_log::info!("daemon offline; starting before preview");
        if let Err(e) = start_daemon_service() {
            idle_log::warn!("could not start daemon for preview: {e:#}");
        }
    }

    if is_running() {
        match TranceClient::connect()
            .context("connect for preview")
            .and_then(|c| c.preview(name).context("D-Bus preview"))
        {
            Ok(()) => return Ok(()),
            Err(e) => idle_log::warn!("D-Bus preview failed: {e:#}; trying run-plugin fallback"),
        }
    }

    for bin in ["idle-daemon", "idlescreen-daemon", "trance-daemon"] {
        if let Ok(mut child) = Command::new(bin).args(["run-plugin", name]).spawn() {
            std::thread::sleep(Duration::from_millis(200));
            if let Ok(Some(status)) = child.try_wait()
                && !status.success()
            {
                bail!("preview process exited early with status: {}", status);
            }
            return Ok(());
        }
    }
    bail!("could not spawn idle-daemon run-plugin for preview")
}

/// Launch the IdleScreen TUI in a terminal (requires a real TTY).
pub fn open_tui_dashboard() -> bool {
    let programs: &[&[&str]] = &[
        &["idle-tui"],
        &["idlescreen-tui"],
        &["idlescreen", "tui"],
        &["idle", "tui"],
    ];
    // cosmic-term: `-e` / `--` then program + args
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
                // SAFETY: start a new session so the terminal outlives the applet.
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
