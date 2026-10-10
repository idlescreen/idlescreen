// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Multi-call binary entry point for IdleScreen screensaver suite.

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
