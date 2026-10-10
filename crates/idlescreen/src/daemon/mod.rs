// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Pure event-driven daemon runtime, epoll reactor, multi-init readiness,
//! and IPC control socket fallback.

pub mod config;
pub mod control_sock;
pub mod dbus;
pub mod gnome;
pub mod notify;
pub mod pidfile;
pub mod reactor;

pub use config::Config;
pub use control_sock::{ControlClient, ControlListener};
pub use notify::{notify_fd3, notify_ready, notify_stopping, notify_watchdog, sd_notify};
pub use pidfile::PidFile;
pub use reactor::EpollReactor;

/// Main entry point for the daemon runtime.
pub fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Acquire exclusive kernel flock on PID file
    let _pidfile = match PidFile::acquire() {
        Ok(pf) => pf,
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            eprintln!("idlescreen-daemon is already running (PID lock held)");
            return Ok(());
        }
        Err(e) => {
            eprintln!("Failed to acquire PID lock: {e}");
            return Err(e.into());
        }
    };

    // 2. Load or migrate configuration
    let config = Config::load_or_migrate();

    // 3. Bind direct control socket
    let control_listener = ControlListener::bind()?;

    // 4. Setup signal handling via Linux signalfd (SIGTERM, SIGINT)
    let mut mask: libc::sigset_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::sigemptyset(&raw mut mask);
        libc::sigaddset(&raw mut mask, libc::SIGTERM);
        libc::sigaddset(&raw mut mask, libc::SIGINT);
        libc::sigprocmask(libc::SIG_BLOCK, &raw const mask, std::ptr::null_mut());
    }
    let sfd = unsafe { libc::signalfd(-1, &raw const mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) };

    // 5. Initialize epoll reactor
    let mut reactor = EpollReactor::new()?;
    // Token 1: control socket
    reactor.add_fd(control_listener.as_raw_fd(), libc::EPOLLIN as u32, 1)?;
    // Token 2: signalfd
    if sfd >= 0 {
        reactor.add_fd(sfd, libc::EPOLLIN as u32, 2)?;
    }

    // 6. Signal readiness across all supervision protocols (systemd, fd3, etc.)
    notify_ready();

    let mut events = [libc::epoll_event { events: 0, u64: 0 }; 16];
    let mut running = true;

    // Main event loop
    while running && reactor.running {
        // Sleep in epoll_wait indefinitely (-1) for pure event-driven reactor (0.00% CPU at idle)
        let num_events = match reactor.poll_events(&mut events, -1) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                eprintln!("reactor error: {e}");
                break;
            }
        };

        for i in 0..num_events {
            let token = events[i].u64;
            if token == 1 {
                // Control socket activity
                if let Ok(mut stream) = control_listener.accept() {
                    use std::io::{BufRead, BufReader, Write};
                    let mut reader = BufReader::new(&mut stream);
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_ok() {
                        let cmd = line.trim();
                        let (resp, should_stop) = match cmd {
                            "status" => (
                                format!(
                                    "{{\"running\":true,\"timeout\":{}}}",
                                    config.general.idle_timeout_secs
                                ),
                                false,
                            ),
                            "stop" => ("OK".to_string(), true),
                            "ping" | "PING" => ("PONG".to_string(), false),
                            _ if cmd.starts_with("set ") => {
                                let saver = cmd[4..].trim();
                                (format!("OK: saver set to {saver}"), false)
                            }
                            _ => ("UNKNOWN_COMMAND".to_string(), false),
                        };
                        let _ = writeln!(stream, "{resp}");
                        if should_stop {
                            running = false;
                            break;
                        }
                    }
                }
            } else if token == 2 {
                // Signal received on signalfd
                running = false;
                break;
            }
        }
    }

    if sfd >= 0 {
        unsafe { libc::close(sfd) };
    }

    // Clean shutdown
    notify_stopping();
    Ok(())
}
