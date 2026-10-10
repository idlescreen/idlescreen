// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Queries daemon status over D-Bus with control.sock fallback.

use crate::daemon::control_sock::ControlClient;

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Try D-Bus first
    if let Ok(conn) = zbus::blocking::Connection::session()
        && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
        && let Ok(status) = proxy.get_status()
    {
        println!("IdleScreen Daemon: active (via D-Bus)");
        println!("Status: {status}");
        return Ok(());
    }

    // 2. Fallback to control socket
    if let Ok(resp) = ControlClient::send_command("status") {
        println!("IdleScreen Daemon: active (via control socket)");
        println!("Response: {resp}");
        Ok(())
    } else {
        // 3. Fallback to PID file (must verify lock is held)
        if let Ok(pid_path) = crate::daemon::pidfile::resolve_pid_path()
            && pid_path.exists()
            && crate::daemon::pidfile::is_daemon_running(&pid_path)
            && let Ok(content) = std::fs::read_to_string(&pid_path)
        {
            println!("IdleScreen Daemon: running (PID {})", content.trim());
            return Ok(());
        }
        println!("IdleScreen Daemon: inactive (not running)");
        Ok(())
    }
}
