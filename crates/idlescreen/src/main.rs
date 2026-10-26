// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Multi-call binary entry point for IdleScreen screensaver suite.

#![deny(unsafe_code)]

// The binary is the desktop product: it drives the daemon, the TUI and the
// CLI, all of which live behind the `product` feature.
#[cfg(not(feature = "product"))]
compile_error!("the idlescreen binary requires the `product` feature");

use std::path::Path;

/// `EX_CONFIG` (sysexits.h): the configuration is unusable.
pub const EX_CONFIG: i32 = 78;
/// `EX_UNAVAILABLE` (sysexits.h): a required service, socket or device is
/// not available.
pub const EX_UNAVAILABLE: i32 = 69;
/// `EX_SOFTWARE` (sysexits.h): an internal error.
pub const EX_SOFTWARE: i32 = 70;

fn main() {
    if let Err(e) = run() {
        let message = e.to_string();
        eprintln!("idlescreen: {message}");
        std::process::exit(exit_code(&message));
    }
}

/// Maps a failure message to a sysexits.h exit code.
fn exit_code(message: &str) -> i32 {
    let lower = message.to_ascii_lowercase();
    if lower.contains("config") {
        EX_CONFIG
    } else if lower.contains("unavailable")
        || lower.contains("not found")
        || lower.contains("no such")
        || lower.contains("not running")
    {
        EX_UNAVAILABLE
    } else {
        EX_SOFTWARE
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let bin_name = args.first().map_or("idlescreen", |s| {
        Path::new(s)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(s)
    });

    match bin_name {
        "idlescreen-daemon" | "idle-daemon" => idlescreen::daemon::run_daemon(),
        "idlescreen-tui" | "idle-tui" => idlescreen::tui::run_tui(),
        _ => {
            let cli_args = if args.len() > 1 { &args[1..] } else { &[] };
            idlescreen::cli::run_cli(cli_args)
        }
    }
}
