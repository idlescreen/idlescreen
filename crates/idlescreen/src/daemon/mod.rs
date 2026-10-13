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

    // 1b. Check if an active daemon already owns our unique D-Bus interface
    if let Ok(conn) = zbus::blocking::Connection::session()
        && let Ok(reply) = conn.call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "NameHasOwner",
            &("io.github.idlescreen.Idle",),
        )
        && let Ok(has_owner) = reply.body().deserialize::<bool>()
        && has_owner
    {
        eprintln!(
            "idlescreen-daemon is already running (D-Bus name io.github.idlescreen.Idle is already owned)"
        );
        return Ok(());
    }

    // 2. Load or migrate configuration
    let config = Config::load_or_migrate();

    // 3. Bind direct control socket
    let control_listener = match ControlListener::bind() {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("idlescreen-daemon is already running (control socket active)");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

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

    let idle_timeout_ms = config.general.idle_timeout_secs * 1000;

    let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
    let wake_handle = crate::daemon::gnome::WakeHandle::new(wake_fd);
    if wake_fd >= 0 {
        state.set_wake_handle(wake_handle.clone());
    }
    let wake_registered = if wake_fd >= 0 {
        reactor.add_fd(wake_fd, libc::EPOLLIN as u32, 3).is_ok()
    } else {
        false
    };

    // Spawn background watchers for GNOME ScreenSaver (ActiveChanged, WakeUpScreen) and logind (PrepareForSleep)
    crate::daemon::gnome::spawn_gnome_session_watchers(state.clone(), wake_handle.clone());

    // Register Mutter IdleMonitor watch signals to eliminate periodic polling wakeups
    let mutter_idle_active = wake_registered
        && crate::daemon::gnome::spawn_mutter_idle_watcher(
            state.clone(),
            idle_timeout_ms,
            wake_handle.clone(),
        );

    let mut events = [libc::epoll_event { events: 0, u64: 0 }; 16];
    let mut running = true;
    let mut presenter_handle: Option<std::thread::JoinHandle<()>> = None;
    let watchdog_active = std::env::var("WATCHDOG_USEC").is_ok();

    // Mutter polling is only useful on GNOME without signal watches. Probe once:
    // on Hyprland/sway the proxy exists but every call fails, so polling it
    // would wake the CPU 4x/s for nothing.
    let mutter_poll_available = !mutter_idle_active
        && mutter_proxy
            .as_ref()
            .is_some_and(|m| m.get_idletime().is_ok());

    // Main event loop
    while running && reactor.running {
        // Event-driven sleep (-1) unless we must poll Mutter; 4s when a systemd watchdog is armed.
        let poll_timeout = if mutter_poll_available {
            250
        } else if watchdog_active {
            4000
        } else {
            -1
        };
        let num_events = match reactor.poll_events(&mut events, poll_timeout) {
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
                                // Stop the active screensaver only; the daemon keeps running.
                                state.stop();
                                ("OK".to_string(), false)
                            }
                            "quit" => {
                                state.stop();
                                ("OK: daemon exiting".to_string(), true)
                            }
                            "ping" | "PING" => ("PONG".to_string(), false),
                            _ if cmd.starts_with("set ") => {
                                let saver = cmd[4..].trim();
                                state.set_saver(saver.to_string());
                                (format!("OK: saver set to {saver}"), false)
                            }
                            _ if cmd.starts_with("text_targets ") => {
                                let targets_str = cmd["text_targets ".len()..].trim();
                                let targets: Vec<String> = targets_str
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                state.set_text_targets(&targets);
                                ("OK: text targets updated".to_string(), false)
                            }
                            _ if cmd.starts_with("set_text ") => {
                                let text = cmd["set_text ".len()..].trim();
                                state.set_text(text);
                                ("OK: custom text updated".to_string(), false)
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
            } else if token == 3 {
                // Wakeup from Mutter idle monitor or session watcher
                let mut buf = 0u64;
                unsafe {
                    libc::read(
                        wake_fd,
                        (&raw mut buf).cast::<libc::c_void>(),
                        std::mem::size_of::<u64>(),
                    );
                }
            }
        }

        // Periodic Mutter idle and lock check on GNOME (fallback when signal watches not available)
        if mutter_poll_available {
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
                    // User activity or screen lock detected
                    state.stop();
                }
            }
        }

        // Reap finished presenter thread when inactive
        if presenter_handle
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
        {
            let res = presenter_handle.take().unwrap().join();
            if let Err(panic_payload) = res {
                eprintln!(
                    "[idlescreen::daemon] Wayland presenter thread panicked: {panic_payload:?}"
                );
                state.stop();
            }
        }

        // Launch Wayland screensaver presentation when active
        if state.active.load(std::sync::atomic::Ordering::SeqCst) {
            let needs_spawn = presenter_handle
                .as_ref()
                .is_none_or(std::thread::JoinHandle::is_finished);
            if needs_spawn {
                if let Some(h) = presenter_handle.take() {
                    let res = h.join();
                    if let Err(panic_payload) = res {
                        eprintln!(
                            "[idlescreen::daemon] Wayland presenter thread panicked: {panic_payload:?}"
                        );
                        state.stop();
                    }
                }
                if !state.active.load(std::sync::atomic::Ordering::SeqCst) {
                    continue;
                }
                let st = state.clone();
                let dwell = config.saver.ascii.dwell_secs;
                let mode = config.saver.ascii.mode.clone();
                let text = st
                    .custom_text
                    .lock()
                    .map_or_else(|_| config.saver.ascii.text.clone(), |t| t.clone());
                let fps = config.saver.ascii.fps;
                presenter_handle = Some(std::thread::spawn(move || {
                    if let Err(err) = crate::wayland::run_wayland_daemon_presenter(
                        st.clone(),
                        dwell,
                        &mode,
                        &text,
                        fps,
                    ) {
                        eprintln!("[idlescreen::daemon] Wayland presenter error: {err}");
                        st.stop();
                    }
                }));
            }
        }
    }

    // Clean shutdown - notify systemd immediately upon exit signal
    notify_stopping();
    state.clear_wake_handle();
    state.stop();
    if let Some(h) = presenter_handle.take() {
        let res = h.join();
        if let Err(panic_payload) = res {
            eprintln!("[idlescreen::daemon] Wayland presenter thread panicked: {panic_payload:?}");
        }
    }

    if sfd >= 0 {
        unsafe { libc::close(sfd) };
    }
    wake_handle.close();

    Ok(())
}
