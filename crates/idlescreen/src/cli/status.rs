// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Queries daemon status over D-Bus with control.sock fallback and JSON support.

use crate::daemon::control_sock::ControlClient;

fn is_systemd_service_active() -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "idlescreen.service"])
        .status()
        .is_ok_and(|s| s.success())
        || std::process::Command::new("systemctl")
            .args(["--user", "is-active", "--quiet", "idle-daemon.service"])
            .status()
            .is_ok_and(|s| s.success())
}

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    execute_args(&[])
}

pub fn execute_args(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let json_mode = args.iter().any(|a| a == "--json");
    let service_active = is_systemd_service_active();

    // 1. Try D-Bus first
    if let Ok(conn) = zbus::blocking::Connection::session()
        && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
        && let Ok(status) = proxy.get_status()
    {
        if json_mode {
            if let Some(stripped) = status.trim().strip_suffix('}') {
                println!("{stripped},\"service_active\":{service_active}}}");
            } else {
                println!("{status}");
            }
        } else {
            println!("IdleScreen Daemon: active (via D-Bus)");
            println!("Status: {status}");
            println!(
                "Service (systemd): {}",
                if service_active { "active" } else { "inactive" }
            );
        }
        return Ok(());
    }

    // 2. Fallback to control socket
    if let Ok(resp) = ControlClient::send_command("status") {
        if json_mode {
            // The daemon's response is itself a JSON document: embed it as
            // an escaped string so the result stays parseable.
            let raw = crate::daemon::dbus::json_escape::escape(resp.trim());
            println!(
                r#"{{"active":true,"service_active":{service_active},"saver":"ascii","raw":"{raw}"}}"#
            );
        } else {
            println!("IdleScreen Daemon: active (via control socket)");
            println!("Response: {resp}");
            println!(
                "Service (systemd): {}",
                if service_active { "active" } else { "inactive" }
            );
        }
        return Ok(());
    }

    // 3. Fallback to PID file (must verify lock is held)
    if let Ok(pid_path) = crate::daemon::pidfile::resolve_pid_path()
        && pid_path.exists()
        && crate::daemon::pidfile::is_daemon_running(&pid_path)
        && let Ok(content) = std::fs::read_to_string(&pid_path)
    {
        if json_mode {
            let pid = content.trim();
            println!(r#"{{"active":true,"service_active":{service_active},"pid":"{pid}"}}"#);
        } else {
            println!("IdleScreen Daemon: running (PID {})", content.trim());
            println!(
                "Service (systemd): {}",
                if service_active { "active" } else { "inactive" }
            );
        }
        return Ok(());
    }

    if json_mode {
        println!(
            r#"{{"active":false,"service_active":{service_active},"saver":"ascii","inhibited":false}}"#
        );
    } else {
        println!("IdleScreen Daemon: inactive (not running)");
        println!(
            "Service (systemd): {}",
            if service_active { "active" } else { "inactive" }
        );
    }
    Ok(())
}
