// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Command line interface subcommands and dispatcher.

pub mod doctor;
pub mod list;
pub mod preview;
pub mod set;
pub mod status;

use crate::daemon::control_sock::ControlClient;

pub fn print_help() {
    println!("idlescreen — High-performance screensaver and idle management suite for Linux");
    println!();
    println!("Usage:");
    println!("  idlescreen <command> [options]");
    println!();
    println!("Commands:");
    println!(
        "  daemon            Run event-driven screensaver daemon (or start idlescreen-daemon)"
    );
    println!("  status            Check active screensaver and daemon status");
    println!("  tui               Interactive settings dashboard for screensaver and timeouts");
    println!("  preview [saver]   Preview screensaver in terminal (crossterm)");
    println!("  list              List available built-in and extras screensavers");
    println!("  set <saver>       Set active screensaver (e.g. ascii, storm, aurora)");
    println!("  start             Activate screensaver immediately");
    println!("  stop              Stop running screensaver");
    println!("  enable            Enable and start IdleScreen service via systemd");
    println!("  disable           Disable and stop IdleScreen service via systemd");
    println!("  restart           Restart IdleScreen service via systemd");
    println!("  timeout [mins]    Get or set idle timeout in minutes");
    println!("  config            Show configuration path and active TOML settings");
    println!("  doctor [options]  Run diagnostic check on display, sockets, and configuration");
    println!("  help              Show this help message");
    println!("  version           Show version information");
}

pub fn run_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "daemon" => crate::daemon::run_daemon(),
        "status" => status::execute(),
        "tui" | "ui" => crate::tui::run_tui(),
        "preview" => {
            let preview_args = if args.len() > 1 { &args[1..] } else { &[] };
            preview::execute(preview_args)
        }
        "list" => list::execute(),
        "set" => {
            let saver = args.get(1).map(std::string::String::as_str);
            set::execute(saver)
        }
        "saver" => match args.get(1).map(std::string::String::as_str) {
            Some("list") => list::execute(),
            Some("set") => {
                let saver = args.get(2).map(std::string::String::as_str);
                set::execute(saver)
            }
            _ => {
                println!("Usage: idlescreen saver <list|set <name>>");
                Ok(())
            }
        },
        "start" => {
            let mut started = false;
            if let Ok(conn) = zbus::blocking::Connection::session()
                && conn
                    .call_method(
                        Some("org.freedesktop.ScreenSaver"),
                        "/org/freedesktop/ScreenSaver",
                        Some("org.freedesktop.ScreenSaver"),
                        "Lock",
                        &(),
                    )
                    .is_ok()
            {
                println!("Daemon response: Triggered screensaver lock (via D-Bus)");
                started = true;
            }
            if !started {
                println!("Daemon not active; running interactive preview instead:");
                let preview_args = if args.len() > 1 { &args[1..] } else { &[] };
                preview::execute(preview_args)?;
            }
            Ok(())
        }
        "enable" => {
            let status = std::process::Command::new("systemctl")
                .args(["--user", "enable", "--now", "idlescreen.service"])
                .status()
                .or_else(|_| {
                    std::process::Command::new("systemctl")
                        .args(["--user", "enable", "--now", "idle-daemon.service"])
                        .status()
                });
            match status {
                Ok(s) if s.success() => println!("IdleScreen service enabled and started."),
                _ => println!("Configured systemd service enable command attempted."),
            }
            Ok(())
        }
        "disable" => {
            let status = std::process::Command::new("systemctl")
                .args(["--user", "disable", "--now", "idlescreen.service"])
                .status()
                .or_else(|_| {
                    std::process::Command::new("systemctl")
                        .args(["--user", "disable", "--now", "idle-daemon.service"])
                        .status()
                });
            match status {
                Ok(s) if s.success() => println!("IdleScreen service disabled and stopped."),
                _ => println!("Configured systemd service disable command attempted."),
            }
            Ok(())
        }
        "restart" => {
            let status = std::process::Command::new("systemctl")
                .args(["--user", "restart", "idlescreen.service"])
                .status()
                .or_else(|_| {
                    std::process::Command::new("systemctl")
                        .args(["--user", "restart", "idle-daemon.service"])
                        .status()
                });
            match status {
                Ok(s) if s.success() => println!("IdleScreen service restarted."),
                _ => println!("Configured systemd service restart command attempted."),
            }
            Ok(())
        }
        "timeout" => {
            let arg = args.get(1).map(std::string::String::as_str);
            handle_timeout(arg)
        }
        "config" => handle_config(),
        "doctor" => {
            let doctor_args = if args.len() > 1 { &args[1..] } else { &[] };
            doctor::execute_args(doctor_args)
        }
        "stop" => {
            let mut stopped = false;
            if let Ok(conn) = zbus::blocking::Connection::session()
                && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
                && proxy.stop().is_ok()
            {
                println!("Daemon response: Stopped active screensaver (via D-Bus)");
                stopped = true;
            }
            if !stopped {
                match ControlClient::send_command("stop") {
                    Ok(resp) => println!("Daemon response: {resp}"),
                    Err(e) => eprintln!("Failed to communicate with daemon: {e}"),
                }
            }
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("idlescreen {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("Unknown command: {other}");
            eprintln!("Run 'idlescreen help' for usage.");
            std::process::exit(1);
        }
    }
}

fn handle_timeout(arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = crate::daemon::config::Config::load_or_migrate();
    if let Some(val_str) = arg {
        let mins: u64 = val_str
            .parse()
            .map_err(|_| "Invalid number of minutes for timeout")?;
        let secs = (mins * 60).clamp(30, 7200);
        config.general.idle_timeout_secs = secs;
        let cfg_dir = crate::daemon::config::resolve_config_dir();
        let toml_path = cfg_dir.join("config.toml");
        let toml_str = toml::to_string_pretty(&config)?;
        let _ = std::fs::create_dir_all(&cfg_dir);
        let _ = std::fs::write(&toml_path, toml_str);
        println!("Idle timeout set to {mins} minute(s) ({secs}s).");
    } else {
        let secs = config.general.idle_timeout_secs;
        let mins = secs / 60;
        let rem = secs % 60;
        println!("Current idle timeout: {mins}m {rem}s ({secs} seconds)");
    }
    Ok(())
}

fn handle_config() -> Result<(), Box<dyn std::error::Error>> {
    let cfg_dir = crate::daemon::config::resolve_config_dir();
    let toml_path = cfg_dir.join("config.toml");
    println!("Config file: {}", toml_path.display());
    if toml_path.exists() {
        if let Ok(c) = std::fs::read_to_string(&toml_path) {
            println!("\n{c}");
        }
    } else {
        println!("(No active config.toml on disk; using defaults)");
    }
    Ok(())
}
