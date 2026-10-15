// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Command line interface subcommands and dispatcher.

pub mod doctor;
pub mod inhibitors;
pub mod list;
pub mod preview;
pub mod service;
pub mod set;
pub mod status;
pub mod text;

use crate::daemon::control_sock::ControlClient;

pub fn print_help() {
    println!("idlescreen — High-performance screensaver and idle management suite for Linux");
    println!();
    println!("Usage:");
    println!("  idlescreen <command> [options]");
    println!();
    println!("Commands:");
    println!("  daemon            Run event-driven screensaver daemon");
    println!("  status [--json]   Check active screensaver and daemon status");
    println!("  inhibitors        List active screensaver inhibitors (--json supported)");
    println!("  tui               Interactive settings dashboard for screensaver and timeouts");
    println!("  preview [saver]   Preview screensaver on Wayland display or in terminal");
    println!("  text [targets]    Set or inspect ASCII text targets (e.g. os, custom, status)");
    println!("  list              List available 50-scene 16-bit arcade screensavers");
    println!("  set <saver>       Set active screensaver (e.g. ascii, tour, aurora, plasma)");
    println!(
        "  start [options]   Activate screensaver immediately (--on-idle respects inhibitors)"
    );
    println!("  stop              Stop running screensaver");
    println!("  toggle            Toggle stay-awake standby (enable/disable idle service)");
    println!("  enable            Enable and start IdleScreen service");
    println!("  disable           Disable and stop IdleScreen service");
    println!("  restart           Restart IdleScreen service");
    println!("  timeout [mins]    Get or set idle timeout in minutes");
    println!("  config            Show configuration path and active TOML settings");
    println!("  doctor [options]  Run diagnostic check (--fix, --verbose, --json)");
    println!("  uninstall         Uninstall IdleScreen and clean up configuration");
    println!("  help              Show this help message");
    println!("  version           Show version information");
}

pub fn run_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    let sub_args = if args.len() > 1 { &args[1..] } else { &[] };
    match args[0].as_str() {
        "daemon" => crate::daemon::run_daemon(),
        "status" => status::execute_args(sub_args),
        "inhibitors" => inhibitors::execute_args(sub_args),
        "tui" | "ui" => crate::tui::run_tui(),
        "preview" => preview::execute(sub_args),
        "text" => text::execute(sub_args),
        "list" => list::execute(),
        "set" => set::execute(args.get(1).map(String::as_str)),
        "saver" => match args.get(1).map(String::as_str) {
            Some("list") => list::execute(),
            Some("set") => set::execute(args.get(2).map(String::as_str)),
            _ => {
                println!("Usage: idlescreen saver <list|set <name>>");
                Ok(())
            }
        },
        "start" => handle_start(sub_args),
        "stop" => handle_stop(),
        "toggle" => service::toggle_service(),
        "enable" => service::enable_service(),
        "disable" => service::disable_service(),
        "restart" => service::restart_service(),
        "timeout" => service::handle_timeout(args.get(1).map(String::as_str)),
        "config" => service::handle_config(),
        "doctor" => doctor::execute_args(sub_args),
        "uninstall" | "remove" => service::handle_uninstall(),
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

fn handle_start(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let respect_inhibitors = sub_args
        .iter()
        .any(|a| a == "--on-idle" || a == "--respect-inhibitors");
    let force = !respect_inhibitors || sub_args.iter().any(|a| a == "--force" || a == "-f");
    let sock_cmd = if force {
        "start --force"
    } else {
        "start --on-idle"
    };

    if let Ok(resp) = ControlClient::send_command(sock_cmd) {
        if resp.contains("inhibited") {
            println!("Daemon response: Screensaver inhibited (active media or inhibitor)");
        } else {
            println!("Daemon response: Activated screensaver");
        }
        return Ok(());
    }

    let dbus_inhibited = !force
        && zbus::blocking::Connection::session().is_ok_and(|c| {
            crate::daemon::dbus::IdleClientProxyBlocking::new(&c)
                .and_then(|p| p.list_inhibitors())
                .is_ok_and(|l| !l.is_empty())
        });
    if !force
        && (dbus_inhibited
            || crate::daemon::watchers::media_inhibit::is_media_or_system_inhibited())
    {
        println!("Daemon response: Screensaver inhibited (active media or inhibitor)");
        return Ok(());
    }

    let started = zbus::blocking::Connection::session().is_ok_and(|conn| {
        ["io.github.idlescreen.Idle", "org.freedesktop.ScreenSaver"]
            .iter()
            .any(|dest| {
                conn.call_method(
                    Some(*dest),
                    "/org/freedesktop/ScreenSaver",
                    Some("org.freedesktop.ScreenSaver"),
                    "Lock",
                    &(),
                )
                .is_ok()
            })
    });
    if started {
        println!("Daemon response: Activated screensaver");
    } else if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        println!("Daemon not active; running interactive preview instead:");
        preview::execute(sub_args)?;
    } else {
        eprintln!("idlescreen: daemon not reachable; screensaver not started");
        std::process::exit(69);
    }
    Ok(())
}

fn handle_stop() -> Result<(), Box<dyn std::error::Error>> {
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
