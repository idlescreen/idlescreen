// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 UberMetroid

mod config;
mod controller;
mod daemon;
mod dbus_server;
mod inhibit;
mod lock_monitor;
mod presentation;

fn main() -> anyhow::Result<()> {
    use anyhow::Context;
    use tracing_subscriber::prelude::*;

    // SAFETY: Setting TRANCE_SPAN_MODE on the main thread before any other
    // thread is spawned is safe. Other code paths only READ this variable.
    unsafe {
        std::env::set_var("TRANCE_SPAN_MODE", "1");
    }

    // Initialize tracing with journald or stderr fallback
    if std::env::var("JOURNAL_STREAM").is_ok() {
        let filter = tracing_subscriber::EnvFilter::builder()
            .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
            .from_env_lossy();
        let registry = tracing_subscriber::registry()
            .with(filter)
            .with(tracing_journald::layer().context("initializing journald tracing layer")?);
        tracing::subscriber::set_global_default(registry)
            .context("installing journald tracing subscriber")?;
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::builder()
                    .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .init();
    }

    // Register visual theme and system query callbacks for dynamically loaded screensaver plugins
    let _ = trance_api::SYSTEM_INFO_CALLBACK.set(trance_runner::toolkit::sys_info::get_system_info);
    let _ =
        trance_api::PALETTE_CALLBACK.set(trance_runner::toolkit::sys_info::query_current_palette);
    let _ = trance_api::MONITOR_BOUNDS_CALLBACK
        .set(trance_runner::toolkit::sys_info::get_primary_monitor_bounds);
    let _ = trance_api::IS_SECONDARY_MONITOR_CALLBACK
        .set(trance_runner::toolkit::sys_info::is_secondary_monitor);

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let sub = &args[1];
        if sub == "run-plugin" {
            if args.len() < 3 {
                eprintln!("error: missing saver name.\nusage: trance-daemon run-plugin <saver>");
                std::process::exit(1);
            }
            let name = &args[2];
            if name.contains('/') || name.contains('\\') {
                eprintln!("error: saver name must not be a path");
                std::process::exit(1);
            }
            let path = trance_runner::launcher::resolve_saver_binary(
                name,
                &trance_runner::launcher::LaunchMode::Preview,
            )
            .unwrap_or_else(|error| {
                eprintln!("error: {error}");
                std::process::exit(1);
            });
            match trance_runner::trance_runner::run_plugin_fullscreen(
                path.to_string_lossy().as_ref(),
            ) {
                Ok(code) => std::process::exit(code as i32),
                Err(e) => {
                    eprintln!("failed to execute screensaver plugin: {}", e);
                    std::process::exit(1);
                }
            }
        } else if sub == "daemon" || sub == "--daemon" {
            daemon::run_daemon()?;
        } else if sub == "--help" || sub == "-h" {
            println!(
                "trance-daemon — background idle monitoring service for trance

usage:
  trance-daemon                     run the background idle daemon (default)
  trance-daemon daemon | --daemon   run the background idle daemon
  trance-daemon run-plugin <saver>  run a trusted screensaver plugin fullscreen
  trance-daemon --help | -h         show this help message"
            );
        } else {
            eprintln!("unknown argument: {}\ntry --help", sub);
            std::process::exit(1);
        }
    } else {
        // Run the daemon by default
        daemon::run_daemon()?;
    }
    Ok(())
}
