// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Universal Linux service management (systemd, dinit, OpenRC, runit, s6, XDG autostart)
//! and runtime configuration subcommands (`timeout`, `config`, `uninstall`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::daemon::config::{Config, resolve_config_dir};
use crate::daemon::control_sock::ControlClient;

fn has_systemd_user() -> bool {
    Path::new("/run/systemd/system").exists()
}

fn autostart_desktop_path() -> PathBuf {
    if let Ok(cfg_home) = std::env::var("XDG_CONFIG_HOME")
        && !cfg_home.trim().is_empty()
    {
        return PathBuf::from(cfg_home).join("autostart/idlescreen.desktop");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config/autostart/idlescreen.desktop");
    }
    PathBuf::from("/tmp/idlescreen.desktop")
}

fn write_xdg_autostart() -> std::io::Result<()> {
    let path = autostart_desktop_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = "[Desktop Entry]\nType=Application\nName=IdleScreen Daemon\nExec=idlescreen daemon\nOnlyShowIn=GNOME;KDE;Hyprland;sway;COSMIC;XFCE;LXQt;Wayfire;niri;\nX-GNOME-Autostart-enabled=true\n";
    fs::write(path, content)
}

fn spawn_background_daemon() -> bool {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("idlescreen"));
    Command::new(exe)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

pub fn is_service_active() -> bool {
    if has_systemd_user() {
        let sysd = Command::new("systemctl")
            .args(["--user", "is-active", "--quiet", "idlescreen.service"])
            .status()
            .is_ok_and(|s| s.success())
            || Command::new("systemctl")
                .args(["--user", "is-active", "--quiet", "idle-daemon.service"])
                .status()
                .is_ok_and(|s| s.success());
        if sysd {
            return true;
        }
    }
    ControlClient::send_command("ping").is_ok_and(|r| r == "PONG")
}

pub fn enable_service_quiet() {
    if has_systemd_user() {
        let status = Command::new("systemctl")
            .args(["--user", "enable", "--now", "idlescreen.service"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .or_else(|_| {
                Command::new("systemctl")
                    .args(["--user", "enable", "--now", "idle-daemon.service"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
            });
        if status.is_ok_and(|s| s.success()) {
            return;
        }
    }
    if Command::new("dinitctl")
        .args(["enable", "idlescreen"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        return;
    }
    let _ = write_xdg_autostart();
    if ControlClient::send_command("ping").map_or(true, |r| r != "PONG") {
        let _ = spawn_background_daemon();
    }
}

pub fn enable_service() -> Result<(), Box<dyn std::error::Error>> {
    enable_service_quiet();
    println!("IdleScreen service enabled and started.");
    Ok(())
}

pub fn disable_service_quiet() {
    if has_systemd_user() {
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "--now", "idlescreen.service"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .or_else(|_| {
                Command::new("systemctl")
                    .args(["--user", "disable", "--now", "idle-daemon.service"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
            });
    }
    let _ = Command::new("dinitctl")
        .args(["disable", "idlescreen"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = fs::remove_file(autostart_desktop_path());
    let _ = ControlClient::send_command("quit");
}

pub fn disable_service() -> Result<(), Box<dyn std::error::Error>> {
    disable_service_quiet();
    println!("IdleScreen service disabled and stopped.");
    Ok(())
}

pub fn restart_service() -> Result<(), Box<dyn std::error::Error>> {
    if has_systemd_user() {
        let status = Command::new("systemctl")
            .args(["--user", "restart", "idlescreen.service"])
            .status()
            .or_else(|_| {
                Command::new("systemctl")
                    .args(["--user", "restart", "idle-daemon.service"])
                    .status()
            });
        if status.is_ok_and(|s| s.success()) {
            println!("IdleScreen service restarted (systemd).");
            return Ok(());
        }
    }
    if Command::new("dinitctl")
        .args(["restart", "idlescreen"])
        .status()
        .is_ok_and(|s| s.success())
    {
        println!("IdleScreen service restarted (dinit).");
        return Ok(());
    }
    let _ = ControlClient::send_command("quit");
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = spawn_background_daemon();
    println!("IdleScreen daemon restarted.");
    Ok(())
}

pub fn toggle_service_quiet() -> bool {
    if is_service_active() {
        disable_service_quiet();
        false
    } else {
        enable_service_quiet();
        true
    }
}

pub fn toggle_service() -> Result<(), Box<dyn std::error::Error>> {
    if toggle_service_quiet() {
        println!("IdleScreen standby: Service enabled (idle monitoring ACTIVE)");
    } else {
        println!("IdleScreen standby: Service disabled (stay-awake ON)");
    }
    Ok(())
}

pub fn handle_timeout(arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::load_or_migrate();
    if let Some(val_str) = arg {
        let mins: u64 = val_str
            .parse()
            .map_err(|_| "Invalid number of minutes for timeout")?;
        let secs = (mins * 60).clamp(30, 7200);
        config.general.idle_timeout_secs = secs;
        let toml_path = resolve_config_dir().join("config.toml");
        config.save_to_path(&toml_path)?;

        let mut notified = false;
        if let Ok(conn) = zbus::blocking::Connection::session()
            && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
            && proxy.set_timeout(secs).is_ok()
        {
            notified = true;
        }
        if !notified {
            let _ = ControlClient::send_command(&format!("timeout {secs}"));
        }
        println!("Idle timeout set to {mins} minute(s) ({secs}s).");
    } else {
        let secs = config.general.idle_timeout_secs;
        println!(
            "Current idle timeout: {}m {}s ({secs} seconds)",
            secs / 60,
            secs % 60
        );
    }
    Ok(())
}

pub fn handle_config() -> Result<(), Box<dyn std::error::Error>> {
    let toml_path = resolve_config_dir().join("config.toml");
    println!("Config file: {}", toml_path.display());
    if toml_path.exists() {
        if let Ok(c) = fs::read_to_string(&toml_path) {
            println!("\n{c}");
        }
    } else {
        println!("(No active config.toml on disk; using defaults)");
    }
    Ok(())
}

pub fn handle_uninstall() -> Result<(), Box<dyn std::error::Error>> {
    println!("IdleScreen removal requested.");
    let _ = disable_service();
    let remove_script = Path::new("/usr/libexec/idlescreen/remove-product-stack");
    if remove_script.exists() {
        println!("Executing product stack uninstaller...");
        if Command::new("sudo")
            .arg(remove_script)
            .status()
            .is_ok_and(|s| s.success())
        {
            println!("IdleScreen product stack and configuration removed successfully.");
        }
    } else {
        println!("To complete removal, run your system package manager:");
        println!("  Fedora/RHEL:   sudo dnf remove idlescreen");
        println!("  Debian/Ubuntu: sudo apt remove idlescreen");
        println!("  Arch Linux:    sudo pacman -R idlescreen");
    }
    Ok(())
}
