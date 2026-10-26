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

fn run_quiet(bin: &str, args: &[&str]) -> bool {
    Command::new(bin)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn systemd_user_either(args: &[&str]) -> bool {
    let mut a1 = vec!["--user"];
    a1.extend_from_slice(args);
    a1.push("idlescreen.service");
    let mut a2 = vec!["--user"];
    a2.extend_from_slice(args);
    a2.push("idle-daemon.service");
    run_quiet("systemctl", &a1) || run_quiet("systemctl", &a2)
}

fn systemd_reload_and_reset() {
    let _ = run_quiet("systemctl", &["--user", "daemon-reload"]);
    let _ = run_quiet(
        "systemctl",
        &["--user", "reset-failed", "idlescreen.service"],
    );
}

fn config_subpath(rel: &str, fallback: &str) -> PathBuf {
    if let Ok(cfg_home) = std::env::var("XDG_CONFIG_HOME")
        && !cfg_home.trim().is_empty()
    {
        return PathBuf::from(cfg_home).join(rel);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join(rel);
    }
    PathBuf::from(fallback)
}

#[rustfmt::skip]
fn autostart_desktop_path() -> PathBuf { config_subpath("autostart/idlescreen.desktop", "/tmp/idlescreen.desktop") }
#[rustfmt::skip]
fn systemd_user_unit_path() -> PathBuf { config_subpath("systemd/user/idlescreen.service", "/tmp/idlescreen.service") }

fn write_xdg_autostart() -> std::io::Result<()> {
    let path = autostart_desktop_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, include_str!("../../../../assets/idlescreen.desktop"))
}

pub fn install_service() -> Result<(), Box<dyn std::error::Error>> {
    if has_systemd_user() {
        let unit_path = systemd_user_unit_path();
        if let Some(parent) = unit_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            &unit_path,
            include_str!("../../../../assets/idlescreen.service"),
        )?;
        systemd_reload_and_reset();
        enable_service_quiet();
        println!(
            "Installed and enabled systemd user unit: {}",
            unit_path.display()
        );
        return Ok(());
    }
    write_xdg_autostart()?;
    enable_service_quiet();
    println!(
        "Installed XDG autostart entry: {}",
        autostart_desktop_path().display()
    );
    Ok(())
}

pub fn handle_service_subcommand(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match sub_args.first().map(String::as_str) {
        Some("install") => install_service(),
        Some("enable" | "start") => enable_service(),
        Some("disable" | "stop") => disable_service(),
        Some("restart" | "reload") => restart_service(),
        Some("toggle") => toggle_service(),
        Some("status") => super::status::execute_args(&sub_args[1..]),
        _ => {
            println!("Usage: idlescreen service <install|enable|disable|restart|toggle|status>");
            Ok(())
        }
    }
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
    (has_systemd_user() && systemd_user_either(&["is-active", "--quiet"]))
        || ControlClient::send_command("ping").is_ok_and(|r| r == "PONG")
}

pub fn enable_service_quiet() {
    if has_systemd_user() {
        systemd_reload_and_reset();
        if systemd_user_either(&["enable", "--now"]) {
            return;
        }
    }
    if run_quiet("dinitctl", &["enable", "idlescreen"]) {
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
        let _ = systemd_user_either(&["disable", "--now"]);
    }
    let _ = run_quiet("dinitctl", &["disable", "idlescreen"]);
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
        systemd_reload_and_reset();
        if systemd_user_either(&["restart"]) {
            println!("IdleScreen service restarted (systemd).");
            return Ok(());
        }
    }
    if run_quiet("dinitctl", &["restart", "idlescreen"]) {
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
    let active = is_service_active();
    if active {
        disable_service_quiet();
    } else {
        enable_service_quiet();
    }
    !active
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
    let mut config = Config::load_checked()?;
    if let Some(val_str) = arg {
        let mins: u64 = val_str
            .parse()
            .map_err(|_| "Invalid number of minutes for timeout")?;
        let secs = (mins * 60).clamp(30, 7200);
        config.general.idle_timeout_secs = secs;
        config.save_to_path(&resolve_config_dir().join("config.toml"))?;
        let notified = zbus::blocking::Connection::session().is_ok_and(|conn| {
            crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
                .is_ok_and(|p| p.set_timeout(secs).is_ok())
        });
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
    if let Ok(c) = fs::read_to_string(&toml_path) {
        println!("\n{c}");
    } else {
        println!("(No active config.toml on disk; using defaults)");
    }
    Ok(())
}

#[rustfmt::skip]
pub fn handle_uninstall() -> Result<(), Box<dyn std::error::Error>> {
    println!("IdleScreen removal requested.");
    let _ = disable_service();
    let remove_script = Path::new("/usr/libexec/idlescreen/remove-product-stack");
    if remove_script.exists() {
        println!("Executing product stack uninstaller...");
        if Command::new("sudo").arg(remove_script).status().is_ok_and(|s| s.success()) {
            println!("IdleScreen product stack and configuration removed successfully.");
        }
    } else {
        println!("To complete removal, run your system package manager:\n  Fedora/RHEL:   sudo dnf remove idlescreen\n  Debian/Ubuntu: sudo apt remove idlescreen\n  Arch Linux:    sudo pacman -R idlescreen");
    }
    Ok(())
}
