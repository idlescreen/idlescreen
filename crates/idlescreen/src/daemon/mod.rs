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
    let sfd =
        unsafe { libc::signalfd(-1, &raw const mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) };

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

    // Shared state across D-Bus, control socket, and supervision
    let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
    if let Some(ref s) = config.saver.active
        && let Ok(mut cur) = state.current_saver.lock()
    {
        (*cur).clone_from(s);
    }

    // Connect to session D-Bus and register interfaces if available
    let dbus_conn = zbus::blocking::connection::Builder::session()
        .ok()
        .and_then(|builder| {
            let conn = builder
                .name("io.github.idlescreen.Idle")
                .ok()?
                .serve_at(
                    "/io/github/idlescreen/Idle",
                    crate::daemon::dbus::IdleInterface {
                        state: state.clone(),
                    },
                )
                .ok()?
                .serve_at(
                    "/org/freedesktop/ScreenSaver",
                    crate::daemon::dbus::ScreenSaverInterface {
                        state: state.clone(),
                    },
                )
                .ok()?
                .build()
                .ok()?;
            // Best-effort registration of generic org.freedesktop.ScreenSaver name.
            // On desktops like GNOME, gsd-screensaver-proxy already owns this name;
            // failure to acquire it is normal and must not prevent idlescreen from running.
            let _ = conn.request_name("org.freedesktop.ScreenSaver");
            Some(conn)
        });

    let mutter_proxy = dbus_conn
        .as_ref()
        .and_then(|conn| crate::daemon::gnome::MutterIdleMonitorProxyBlocking::new(conn).ok());

    let gnome_screensaver_proxy = dbus_conn
        .as_ref()
        .and_then(|conn| crate::daemon::gnome::GnomeScreenSaverProxyBlocking::new(conn).ok());

    // Spawn background watchers for GNOME ScreenSaver (ActiveChanged, WakeUpScreen) and logind (PrepareForSleep)
    crate::daemon::gnome::spawn_gnome_session_watchers(state.clone());

    let mut events = [libc::epoll_event { events: 0, u64: 0 }; 16];
    let mut running = true;
    let mut presenter_handle: Option<std::thread::JoinHandle<()>> = None;
    let idle_timeout_ms = config.general.idle_timeout_secs * 1000;

    // Main event loop
    while running && reactor.running {
        // Sleep in epoll_wait (250ms timeout for snappy idle/activity check and heartbeat)
        let num_events = match reactor.poll_events(&mut events, 250) {
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
                            "status" => {
                                let active = state.active.load(std::sync::atomic::Ordering::SeqCst);
                                let saver = state
                                    .current_saver
                                    .lock()
                                    .map_or_else(|_| "ascii".to_string(), |s| s.clone());
                                (
                                    format!(
                                        "{{\"running\":true,\"active\":{active},\"saver\":\"{saver}\",\"timeout\":{}}}",
                                        config.general.idle_timeout_secs
                                    ),
                                    false,
                                )
                            }
                            "stop" => {
                                state.stop();
                                ("OK".to_string(), true)
                            }
                            "ping" | "PING" => ("PONG".to_string(), false),
                            _ if cmd.starts_with("set ") => {
                                let saver = cmd[4..].trim();
                                state.set_saver(saver.to_string());
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

        // Periodic Mutter idle and lock check on GNOME
        let is_locked = gnome_screensaver_proxy
            .as_ref()
            .and_then(|g| g.get_active().ok())
            .unwrap_or(false);

        if let Some(t) = mutter_proxy.as_ref().and_then(|m| m.get_idletime().ok()) {
            if t >= idle_timeout_ms && !state.is_inhibited() && !is_locked {
                state
                    .active
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            } else if (t < idle_timeout_ms || is_locked)
                && state.active.load(std::sync::atomic::Ordering::SeqCst)
            {
                // User activity (trackpad/key movement) or screen lock/DPMS power-off detected.
                // Dismiss active screensaver immediately so it never lingers after unlock.
                state.stop();
            }
        } else if is_locked && state.active.load(std::sync::atomic::Ordering::SeqCst) {
            state.stop();
        }

        // Reap finished presenter thread when inactive
        if let Some(ref h) = presenter_handle {
            if h.is_finished() {
                let _ = presenter_handle.take().unwrap().join();
            }
        }

        // Launch Wayland screensaver presentation when active
        if state.active.load(std::sync::atomic::Ordering::SeqCst) {
            let needs_spawn = presenter_handle
                .as_ref()
                .is_none_or(std::thread::JoinHandle::is_finished);
            if needs_spawn {
                let st = state.clone();
                let dwell = config.saver.ascii.dwell_secs;
                let mode = config.saver.ascii.mode.clone();
                let text = config.saver.ascii.text.clone();
                let fps = config.saver.ascii.fps;
                presenter_handle = Some(std::thread::spawn(move || {
                    let _ =
                        crate::wayland::run_wayland_daemon_presenter(st, dwell, &mode, &text, fps);
                }));
            }
        }
    }

    // Clean shutdown - notify systemd immediately upon exit signal
    notify_stopping();
    state.stop();
    if let Some(h) = presenter_handle.take() {
        let _ = h.join();
    }

    if sfd >= 0 {
        unsafe { libc::close(sfd) };
    }

    Ok(())
}
