// SPDX-License-Identifier: MIT

mod config;
mod controller;
mod daemon;
mod dbus_server;
mod inhibit;
mod lock_monitor;
mod presentation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                eprintln!("error: missing plugin path.\nusage: trance-daemon run-plugin <path>");
                std::process::exit(1);
            }
            let path = &args[2];
            match trance_runner::trance_runner::run_plugin_fullscreen(path) {
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
  trance-daemon run-plugin <path>   run a screensaver plugin fullscreen
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
