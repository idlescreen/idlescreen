// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 UberMetroid

use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use trance_dbus::{TranceClient, daemon_available};

mod bug_report;
mod clean;
mod completion;
mod config;
mod doctor;
mod interactive;
mod self_update;
mod usage;

#[cfg(test)]
mod tests;

fn main() -> ExitCode {
    init_tracing();
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    #[cfg(feature = "journald")]
    {
        use tracing_subscriber::prelude::*;
        if let Ok(layer) = tracing_journald::layer() {
            let _ = tracing_subscriber::registry()
                .with(env_filter.clone())
                .with(layer)
                .try_init();
            return;
        }
    }

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

#[tracing::instrument(skip_all)]
fn run(args: Vec<String>) -> Result<()> {
    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help" | "help") {
        print_usage();
        return Ok(());
    }

    // Version works without a running daemon.
    match args[0].as_str() {
        "version" | "about" | "--version" | "-V" => {
            print_version(args[0].as_str() == "about" || args.get(1).map(String::as_str) == Some("--verbose") || args.get(1).map(String::as_str) == Some("-v"));
            return Ok(());
        }
        _ => {}
    }

    match args[0].as_str() {
        "doctor" => {
            let fix = args.iter().any(|a| a == "--fix" || a == "-f");
            return doctor::run_doctor(fix);
        }
        "clean" => return clean::handle_clean(),
        "completion" => return completion::handle_completion(&args[1..]),
        "bug-report" => return bug_report::handle_bug_report(),
        "self-update" => return self_update::handle_self_update(),
        _ => {}
    }

    let client = if daemon_available() {
        TranceClient::connect().context("failed to connect to daemon")?
    } else {
        bail!("trance-daemon is not running; start it with: systemctl --user start trance-daemon");
    };

    match args[0].as_str() {
        "status" => cmd_status(&client, &args[1..]),
        "config" => config::handle_config(&client, &args[1..]),
        "interactive" => interactive::run_interactive(&client),
        "enable" => client.enable().context("enabling idle screensaver"),
        "disable" => client.disable().context("disabling idle screensaver"),
        "timeout" => cmd_timeout(&client, &args[1..]),
        "saver" => cmd_saver(&client, &args[1..]),
        "list" => cmd_list(&client),
        "inhibitors" => cmd_inhibitors(&client),
        "preview" => cmd_preview(&client, &args[1..]),
        "stop" => client
            .stop_preview()
            .context("stopping preview or idle presentation"),
        "fps-overlay" => cmd_fps_overlay(&client, &args[1..]),
        "render-scale" => cmd_render_scale(&client, &args[1..]),
        _ => {
            print_usage();
            Err(anyhow::anyhow!("unknown command: {}", args[0]))
        }
    }
}

fn cmd_status(client: &TranceClient, args: &[String]) -> Result<()> {
    let status = client.get_status().context("querying daemon status")?;
    if args.first().map(String::as_str) == Some("--json") {
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
    } else {
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
    Ok(())
}

fn cmd_timeout(client: &TranceClient, args: &[String]) -> Result<()> {
    let minutes = match args {
        [value] => value
            .parse::<u32>()
            .context("timeout requires a number of minutes (1–240)")?,
        _ => bail!("usage: trance timeout <minutes>"),
    };
    client.set_timeout(minutes).context("setting idle timeout")
}

fn cmd_saver(client: &TranceClient, args: &[String]) -> Result<()> {
    match args {
        [cmd, name] if cmd == "set" => {
            let dbus_name = if name == "random" { "" } else { name.as_str() };
            client
                .set_saver(dbus_name)
                .context("setting active saver via d-bus")
        }
        [cmd] if cmd == "list" => cmd_list(client),
        _ => bail!("usage: trance saver set <name|random> | trance saver list"),
    }
}

fn cmd_list(client: &TranceClient) -> Result<()> {
    let savers = client
        .list_savers()
        .context("listing installed savers via d-bus")?;
    for saver in savers {
        println!("{saver}");
    }
    Ok(())
}

fn cmd_inhibitors(client: &TranceClient) -> Result<()> {
    let inhibitors = client
        .list_inhibitors()
        .context("listing active inhibitors via d-bus")?;
    if inhibitors.is_empty() {
        println!("No active inhibitors.");
    } else {
        println!("Active inhibitors:");
        for (cookie, app, reason) in inhibitors {
            println!("  [{cookie}] {app}: {reason}");
        }
    }
    Ok(())
}

fn cmd_preview(client: &TranceClient, args: &[String]) -> Result<()> {
    let name = args.first().context("usage: trance preview <saver>")?;
    client.preview(name).context("starting preview via d-bus")
}

fn cmd_fps_overlay(client: &TranceClient, args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().context("querying daemon status")?;
            println!(
                "fps overlay: {}",
                if status.show_fps_overlay { "on" } else { "off" }
            );
            Ok(())
        }
        Some("on") => client
            .set_show_fps_overlay(true)
            .context("enabling fps overlay via d-bus"),
        Some("off") => client
            .set_show_fps_overlay(false)
            .context("disabling fps overlay via d-bus"),
        Some(value) => Err(anyhow::anyhow!(
            "unknown fps-overlay subcommand: {value} (use on, off, status)"
        )),
    }
}

fn cmd_render_scale(client: &TranceClient, args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().context("querying daemon status")?;
            println!(
                "render scale: {}",
                if status.render_scale.is_empty() {
                    "default"
                } else {
                    &status.render_scale
                }
            );
            Ok(())
        }
        Some("default") => client
            .set_render_scale(0.0)
            .context("resetting render scale via d-bus"),
        Some(value) => {
            let scale = value
                .parse::<f32>()
                .context("render-scale requires a number between 0.25 and 1.0")?;
            if !(0.25..=1.0).contains(&scale) {
                bail!("render-scale must be between 0.25 and 1.0");
            }
            client
                .set_render_scale(scale)
                .context("setting render scale via d-bus")
        }
    }
}

fn display_saver(name: &str) -> String {
    if name.is_empty() {
        "random".into()
    } else {
        name.to_string()
    }
}

fn print_usage() {
    usage::print_usage();
}

/// CLI package version from Cargo (matches the shipped `trance-cli` / recommended stack).
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_version(verbose: bool) {
    // Short form (scripts / habit):
    //   trance version
    //   trance --version
    println!("trance {CLI_VERSION}");
    if !verbose {
        return;
    }
    // Longer form:
    //   trance about
    println!("Trance screensaver control CLI");
    println!("License: Apache-2.0");
    println!("Home:    https://github.com/UberMetroid/trance");
    if daemon_available() {
        if let Ok(client) = TranceClient::connect() {
            if let Ok(status) = client.get_status() {
                println!(
                    "Daemon:  reachable ({})",
                    if status.running { "running" } else { "connected" }
                );
                return;
            }
        }
        println!("Daemon:  reachable");
    } else {
        println!("Daemon:  not running");
    }
}
