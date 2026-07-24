// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Status display and version reporting for the idle CLI.

use anyhow::{Context, Result};
use idle_dbus::{TranceClient, daemon_available};

fn display_saver(name: &str) -> String {
    if name.is_empty() {
        "random".into()
    } else {
        name.to_string()
    }
}

fn print_status_json(status: &idle_dbus::DaemonStatus) {
    println!(
        "{{\"running\":{},\"idle_enabled\":{},\"idle_timeout_mins\":{},\"active_saver\":\"{}\",\"gpu_enabled\":{},\"show_fps_overlay\":{},\"render_scale\":\"{}\",\"presentation_active\":{},\"preview_active\":{},\"current_saver\":\"{}\",\"system_idle\":{},\"session_locked\":{},\"inhibited\":{}}}",
        status.running,
        status.idle_enabled,
        status.idle_timeout_mins,
        status.active_saver,
        status.gpu_enabled,
        status.show_fps_overlay,
        status.render_scale,
        status.presentation_active,
        status.preview_active,
        status.current_saver,
        status.system_idle,
        status.session_locked,
        status.inhibited
    );
}

fn print_status_text(status: &idle_dbus::DaemonStatus) {
    println!("running:              {}", status.running);
    println!("idle_enabled:         {}", status.idle_enabled);
    println!("idle_timeout_mins:    {}", status.idle_timeout_mins);
    println!(
        "active_saver:         {}",
        display_saver(&status.active_saver)
    );
    println!("gpu_enabled:          {}", status.gpu_enabled);
    println!("show_fps_overlay:     {}", status.show_fps_overlay);
    println!(
        "render_scale:         {}",
        if status.render_scale.is_empty() {
            "default"
        } else {
            &status.render_scale
        }
    );
    println!("presentation_active:  {}", status.presentation_active);
    println!("preview_active:       {}", status.preview_active);
    println!("current_saver:        {}", status.current_saver);
    println!("system_idle:          {}", status.system_idle);
    println!("session_locked:       {}", status.session_locked);
    println!("inhibited:            {}", status.inhibited);
}

pub fn cmd_status(client: &TranceClient, args: &[String]) -> Result<()> {
    let status = client.get_status().context("querying daemon status")?;
    if args.first().map(String::as_str) == Some("--json") {
        print_status_json(&status);
    } else {
        print_status_text(&status);
    }
    Ok(())
}

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_daemon_version_line() {
    if !daemon_available() {
        println!("Daemon:  not running");
        return;
    }
    if let Ok(client) = TranceClient::connect()
        && let Ok(status) = client.get_status()
    {
        println!(
            "Daemon:  reachable ({})",
            if status.running {
                "running"
            } else {
                "connected"
            }
        );
        return;
    }
    println!("Daemon:  reachable");
}

pub fn print_version(verbose: bool) {
    println!("idle {CLI_VERSION}");
    if !verbose {
        return;
    }
    println!("Trance screensaver control CLI");
    println!("License: Apache-2.0");
    println!("Home:    https://github.com/idlescreen/idle-core");
    if let Some(pkg) = package_version_hint() {
        println!("Package: {pkg}");
    }
    print_daemon_version_line();
}

fn package_hint_from_command(
    program: &str,
    args: &[&str],
    reject_substr: Option<&str>,
) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        return None;
    }
    if let Some(bad) = reject_substr
        && s.contains(bad)
    {
        return None;
    }
    Some(s)
}

fn package_version_hint() -> Option<String> {
    if let Some(s) = package_hint_from_command(
        "rpm",
        &[
            "-q",
            "trance",
            "--qf",
            "%{NAME}-%{VERSION}-%{RELEASE}.%{ARCH}",
        ],
        Some("is not installed"),
    ) {
        return Some(s);
    }
    package_hint_from_command(
        "dpkg-query",
        &["-W", "-f=${Package} ${Version}", "trance"],
        None,
    )
}
