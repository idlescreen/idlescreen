// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Queries active screensaver inhibitors over D-Bus with control.sock fallback and `--json` support.

use crate::daemon::control_sock::ControlClient;

pub fn execute_args(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let json_mode = args.iter().any(|a| a == "--json");

    if let Ok(conn) = zbus::blocking::Connection::session()
        && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
        && let Ok(list) = proxy.list_inhibitors()
    {
        if json_mode {
            let entries: Vec<String> = list
                .iter()
                .map(|(cookie, app, reason)| {
                    let app_esc = app.replace('\\', "\\\\").replace('"', "\\\"");
                    let r_esc = reason.replace('\\', "\\\\").replace('"', "\\\"");
                    format!("{{\"cookie\":{cookie},\"app\":\"{app_esc}\",\"reason\":\"{r_esc}\"}}")
                })
                .collect();
            println!("[{}]", entries.join(","));
        } else if list.is_empty() {
            println!("No active screensaver inhibitors.");
        } else {
            println!("Active screensaver inhibitors ({}):", list.len());
            for (cookie, app, reason) in list {
                println!("  • [{cookie}] {app}: {reason}");
            }
        }
        return Ok(());
    }

    if let Ok(resp) = ControlClient::send_command("inhibitors") {
        if json_mode {
            println!("{resp}");
        } else if resp.trim() == "[]" {
            println!("No active screensaver inhibitors.");
        } else {
            println!("Active screensaver inhibitors (via control socket): {resp}");
        }
        return Ok(());
    }

    let fallback = crate::daemon::watchers::media_inhibit::probe_active_inhibitors();
    if json_mode {
        let entries: Vec<String> = fallback
            .iter()
            .map(|(cookie, app, reason)| {
                let app_esc = app.replace('\\', "\\\\").replace('"', "\\\"");
                let r_esc = reason.replace('\\', "\\\\").replace('"', "\\\"");
                format!("{{\"cookie\":{cookie},\"app\":\"{app_esc}\",\"reason\":\"{r_esc}\"}}")
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if fallback.is_empty() {
        println!("No active screensaver inhibitors (daemon not running).");
    } else {
        println!("Active screensaver inhibitors ({}):", fallback.len());
        for (cookie, app, reason) in fallback {
            println!("  • [{cookie}] {app}: {reason}");
        }
    }
    Ok(())
}
