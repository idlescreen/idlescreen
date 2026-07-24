// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use commands::{
    cmd_fps_overlay, cmd_inhibitors, cmd_list, cmd_preview, cmd_render_scale, cmd_saver,
    cmd_status, cmd_timeout, print_version,
};
use trance_dbus::{TranceClient, daemon_available};

mod bug_report;
mod clean;
mod commands;
mod completion;
mod config;
mod doctor;
mod doctor_checks;
mod doctor_env;
mod doctor_fs;
mod doctor_service;
mod doctor_sys;
mod interactive;
mod self_update;
mod self_update_backend;
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
    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    let head = args[0].as_str();
    let rest = &args[1..];

    match head {
        "-h" | "--help" => {
            print_usage();
            return Ok(());
        }
        "-V" | "--version" => {
            print_version(false);
            return Ok(());
        }
        "-help" => {
            bail!("unknown option: -help (use --help or -h, or: trance help)");
        }
        "-version" => {
            bail!("unknown option: -version (use --version or -V, or: trance version / trance v)");
        }
        _ => {}
    }

    match head {
        "help" => {
            print_usage();
            return Ok(());
        }
        "version" | "v" => {
            print_version(false);
            return Ok(());
        }
        "about" => {
            print_version(true);
            return Ok(());
        }
        "doctor" | "doc" => {
            let fix = rest.iter().any(|a| a == "--fix" || a == "-f");
            let json = rest.iter().any(|a| a == "--json" || a == "-j");
            return doctor::run_doctor(fix, json);
        }
        "clean" => return clean::handle_clean(),
        "completion" => return completion::handle_completion(rest),
        "bug-report" => return bug_report::handle_bug_report(),
        "self-update" | "update" => return self_update::handle_self_update(),
        "tui" => {
            let mut cmd = std::process::Command::new("trance-tui");
            cmd.args(rest);
            let status = cmd.status().context("failed to execute trance-tui")?;
            if status.success() {
                return Ok(());
            } else {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        _ => {}
    }

    let client = if daemon_available() {
        TranceClient::connect().context("failed to connect to daemon")?
    } else {
        bail!("idle-daemon is not running; start it with: systemctl --user start idle-daemon");
    };

    match head {
        "status" | "st" => cmd_status(&client, rest),
        "config" | "cfg" => config::handle_config(&client, rest),
        "interactive" | "i" => interactive::run_interactive(&client),
        "enable" | "on" => client.enable().context("enabling idle screensaver"),
        "disable" | "off" => client.disable().context("disabling idle screensaver"),
        "timeout" | "t" => cmd_timeout(&client, rest),
        "saver" => cmd_saver(&client, rest),
        "list" | "ls" => cmd_list(&client),
        "inhibitors" => cmd_inhibitors(&client),
        "preview" | "p" => cmd_preview(&client, rest),
        "stop" => client
            .stop_preview()
            .context("stopping preview or idle presentation"),
        "fps-overlay" | "fps" => cmd_fps_overlay(&client, rest),
        "render-scale" | "scale" => cmd_render_scale(&client, rest),
        _ => {
            print_usage();
            Err(anyhow::anyhow!("unknown command: {head}"))
        }
    }
}

fn print_usage() {
    usage::print_usage();
}
