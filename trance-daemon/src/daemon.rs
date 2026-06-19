// SPDX-License-Identifier: MIT

use crate::config::DaemonConfig;
use crate::idle::query_logind_idle;
use trance_runner::launcher::{launch_screensaver, LaunchMode, ALLOWED_SAVERS};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn handle_signal(_sig: libc::c_int) {
    RUNNING.store(false, Ordering::Relaxed);
}

pub fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    // Write PID file for process tracking
    let pid_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir).join("trance-daemon.pid")
    } else {
        std::env::temp_dir().join("trance-daemon.pid")
    };
    fs::write(&pid_path, std::process::id().to_string())?;

    // Setup signal handler for graceful shutdown
    unsafe {
        libc::signal(
            libc::SIGINT,
            handle_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            handle_signal as *const () as libc::sighandler_t,
        );
    }

    println!("trance-daemon running (pid {})...", std::process::id());

    let mut config = DaemonConfig::load();
    let mut active_child: Option<trance_runner::launcher::ScreensaverProcess> = None;
    let mut tick_counter = 0;
    let mut last_headless_warn: Option<Instant> = None;

    while RUNNING.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(1000));
        tick_counter += 1;

        // Reload config every 10 seconds to detect timeout adjustments in TUI
        if tick_counter % 10 == 0 {
            config = DaemonConfig::load();
        }

        let current_time_micros = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_micros() as u64,
            Err(_) => 0,
        };

        if config.idle_enabled {
            if let Some(idle) = query_logind_idle() {
                if idle.is_idle && idle.idle_since_micros > 0 {
                    let elapsed_sec =
                        current_time_micros.saturating_sub(idle.idle_since_micros) / 1_000_000;
                    let target_timeout_sec = (config.idle_timeout_mins * 60) as u64;

                    if elapsed_sec >= target_timeout_sec {
                        if active_child.is_none() {
                            let has_display = std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();
                            if has_display {
                                // Choose a screensaver: active_saver if set, else random from allowlist
                                let name = if let Some(ref active) = config.active_saver {
                                    active.clone()
                                } else {
                                    let mut seed = current_time_micros;
                                    seed = seed
                                        .wrapping_mul(6364136223846793005)
                                        .wrapping_add(1442695040888963407);
                                    let rand_idx = (seed % ALLOWED_SAVERS.len() as u64) as usize;
                                    ALLOWED_SAVERS[rand_idx].to_string()
                                };

                                println!(
                                    "system idle ({}s >= {}s). launching screensaver '{}'...",
                                    elapsed_sec, target_timeout_sec, name
                                );
                                match launch_screensaver(&name, LaunchMode::Daemon) {
                                    Ok(child) => {
                                        active_child = Some(child);
                                    }
                                    Err(e) => {
                                        eprintln!("daemon failed to launch screensaver: {}", e);
                                    }
                                }
                            } else {
                                let now = Instant::now();
                                let should_warn = match last_headless_warn {
                                    Some(last) => now.duration_since(last).as_secs() > 60,
                                    None => true,
                                };
                                if should_warn {
                                    eprintln!("daemon warning: system is idle but no graphical display (DISPLAY or WAYLAND_DISPLAY) was detected. skipping screensaver launch.");
                                    last_headless_warn = Some(now);
                                }
                            }
                        } else {
                            // If running, verify it hasn't exited
                            if let Some(ref mut child) = active_child {
                                if let Ok(Some(status)) = child.try_wait() {
                                    println!("screensaver process exited (status: {}). resetting child.", status);
                                    active_child = None;
                                }
                            }
                        }
                    } else {
                        // Idle but timeout not reached yet
                        if let Some(mut child) = active_child.take() {
                            println!("system activity detected (idle duration reset to {}s). killing screensaver...", elapsed_sec);
                            let _ = child.kill();
                        }
                    }
                } else {
                    // Active session (not idle)
                    if let Some(mut child) = active_child.take() {
                        println!(
                            "system activity detected (session active). killing screensaver..."
                        );
                        let _ = child.kill();
                    }
                }
            } else {
                // Query failed (e.g. systemd-logind not running or DBus issue)
                if let Some(mut child) = active_child.take() {
                    let _ = child.kill();
                }
            }
        } else {
            // Idle activation disabled: make sure screensaver is not running!
            if let Some(mut child) = active_child.take() {
                println!("idle activation disabled by user config. killing screensaver...");
                let _ = child.kill();
            }
        }
    }

    // Cleanup on exit
    if let Some(mut child) = active_child {
        let _ = child.kill();
    }
    let _ = fs::remove_file(pid_path);
    println!("daemon shutdown complete.");
    Ok(())
}
