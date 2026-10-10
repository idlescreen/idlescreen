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
    println!("  daemon            Run event-driven screensaver daemon (or start idlescreen-daemon)");
    println!("  status            Check active screensaver and daemon status");
    println!("  preview [saver]   Preview screensaver in terminal (crossterm)");
    println!("  list              List available built-in and extras screensavers");
    println!("  set <saver>       Set active screensaver (e.g. ascii, storm, aurora)");
    println!("  stop              Stop running screensaver");
    println!("  doctor            Run diagnostic check on display, sockets, and configuration");
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
        "preview" => {
            let saver = args.get(1).map(std::string::String::as_str);
            preview::execute(saver)
        }
        "list" => list::execute(),
        "set" => {
            let saver = args.get(1).map(std::string::String::as_str);
            set::execute(saver)
        }
        "doctor" => doctor::execute(),
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
