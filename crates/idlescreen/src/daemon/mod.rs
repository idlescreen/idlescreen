// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Pure event-driven daemon runtime, epoll reactor, multi-init readiness,
//! and IPC control socket fallback.

#![allow(unsafe_code)]

pub mod config;
pub mod control_client;
pub mod control_sock;
pub mod dbus;
pub mod notify;
pub mod pidfile;
pub mod reactor;
pub mod runtime;
pub mod state;
pub mod watchers;
pub use watchers as gnome;

pub use config::Config;
pub use control_sock::{ControlClient, ControlListener};
pub use notify::{
    log_status, notify_fd3, notify_ready, notify_stopping, notify_watchdog, sd_notify,
};
pub use pidfile::PidFile;
pub use reactor::EpollReactor;
pub use runtime::{RuntimeEnvironment, detect_environment};

use std::sync::Arc;
use std::sync::atomic::Ordering;

pub fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let env = detect_environment();
    notify::log_status(&format!("Starting in {env:?} mode"));

    let _pidfile = match PidFile::acquire() {
        Ok(pf) => pf,
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            log_status("idlescreen-daemon is already running (PID lock held)");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    if let Ok(conn) = zbus::blocking::Connection::session()
        && watchers::mutter::has_bus_owner(&conn, "io.github.idlescreen.Idle")
    {
        log_status("idlescreen-daemon is already running (D-Bus name owned)");
        return Ok(());
    }

    let config = match Config::load_checked() {
        Ok(c) => c,
        Err(e) => {
            // Surface the problem instead of silently running on defaults.
            log_status(&format!(
                "config error: {e} — running with built-in defaults"
            ));
            Config::default()
        }
    };
    let control_listener = match ControlListener::bind() {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            log_status("idlescreen-daemon is already running (control socket active)");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    // SAFETY: `sigset_t` is a plain C struct; `sigemptyset`/`sigaddset`/`sigprocmask`/`signalfd` are standard POSIX/Linux calls.
    let sfd = unsafe {
        let mut mask: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&raw mut mask);
        libc::sigaddset(&raw mut mask, libc::SIGTERM);
        libc::sigaddset(&raw mut mask, libc::SIGINT);
        libc::sigprocmask(libc::SIG_BLOCK, &raw const mask, std::ptr::null_mut());
        libc::signalfd(-1, &raw const mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC)
    };

    let mut reactor = EpollReactor::new()?;
    reactor.add_fd(control_listener.as_raw_fd(), libc::EPOLLIN as u32, 1)?;
    if sfd >= 0 {
        reactor.add_fd(sfd, libc::EPOLLIN as u32, 2)?;
    }

    let state = Arc::new(dbus::ScreenSaverState::new());
    if let Some(ref s) = config.saver.active
        && let Ok(mut cur) = state.current_saver.lock()
    {
        (*cur).clone_from(s);
    }

    let dbus_conn = dbus::register_dbus_services(state.clone());
    // Inhibitor probes fork processes; they run on their own thread so the
    // event thread never blocks on them.
    watchers::inhibit_cache::start();

    let wake_handle = watchers::WakeHandle::new_eventfd();
    let cfg_wake = watchers::WakeHandle::new_eventfd();
    if wake_handle.is_valid() {
        state.set_wake_handle(wake_handle.clone());
    }
    if cfg_wake.is_valid() {
        state.set_idle_cfg_wake(cfg_wake.clone());
    }
    let wake_registered = wake_handle.is_valid()
        && reactor
            .add_fd(wake_handle.raw_fd(), libc::EPOLLIN as u32, 3)
            .is_ok();

    watchers::spawn_gnome_session_watchers(state.clone(), wake_handle.clone());

    let wayland_idle_active = wake_registered
        && watchers::spawn_wayland_idle_watcher(
            state.clone(),
            wake_handle.clone(),
            cfg_wake.clone(),
        );
    let mutter_idle_active = !wayland_idle_active
        && wake_registered
        && watchers::spawn_mutter_idle_watcher(
            state.clone(),
            config.general.idle_timeout_secs * 1000,
            wake_handle.clone(),
        );

    let mutter_proxy = dbus_conn
        .as_ref()
        .filter(|c| {
            !wayland_idle_active && !mutter_idle_active && watchers::is_mutter_idle_available(c)
        })
        .and_then(|c| watchers::MutterIdleMonitorProxyBlocking::new(c).ok());
    let gnome_ss_proxy = dbus_conn
        .as_ref()
        .filter(|_| mutter_proxy.is_some())
        .and_then(|c| watchers::GnomeScreenSaverProxyBlocking::new(c).ok());

    notify_ready();

    let mut events = [libc::epoll_event { events: 0, u64: 0 }; 16];
    let mut running = true;
    let mut presenter_handle: Option<std::thread::JoinHandle<()>> = None;
    let watchdog_active = std::env::var("WATCHDOG_USEC").is_ok();

    while running && reactor.running {
        let poll_timeout = if mutter_proxy.is_some() {
            250
        } else if watchdog_active {
            4000
        } else {
            -1
        };
        let num_events = match reactor.poll_events(&mut events, poll_timeout) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };

        if reactor.last_poll_suspended {
            state.session_locked.store(false, Ordering::SeqCst);
            state.stop();
        }

        for ev in &events[..num_events] {
            match ev.u64 {
                1 => {
                    if let Ok(mut stream) = control_listener.accept()
                        && control_client::handle_control_client(&mut stream, &state)
                    {
                        notify_stopping();
                        running = false;
                        break;
                    }
                }
                2 => {
                    notify_stopping();
                    running = false;
                    break;
                }
                3 => wake_handle.drain(),
                _ => {}
            }
        }

        if !running {
            break;
        }

        if let Some(ref mutter) = mutter_proxy {
            watchers::mutter::poll_mutter_fallback(mutter, gnome_ss_proxy.as_ref(), &state);
        }

        sync_presenter_thread(&mut presenter_handle, &state, &config);
    }

    notify_stopping();
    state.clear_wake_handle();
    state.stop();
    if let Some(h) = presenter_handle.take() {
        let _ = h.join();
    }
    if sfd >= 0 {
        // SAFETY: `sfd` is a valid signalfd owned by `run_daemon`.
        unsafe { libc::close(sfd) };
    }
    Ok(())
}

fn sync_presenter_thread(
    presenter_handle: &mut Option<std::thread::JoinHandle<()>>,
    state: &Arc<dbus::ScreenSaverState>,
    config: &Config,
) {
    if presenter_handle
        .as_ref()
        .is_some_and(std::thread::JoinHandle::is_finished)
        && let Some(h) = presenter_handle.take()
        && h.join().is_err()
    {
        state.stop();
    }
    if state.active.load(Ordering::SeqCst) && presenter_handle.is_none() {
        let st = state.clone();
        let fresh = Config::load_or_migrate();
        let dwell = if fresh.saver.ascii.dwell_secs > 0.0 {
            fresh.saver.ascii.dwell_secs
        } else {
            config.saver.ascii.dwell_secs
        };
        let mode = fresh.saver.ascii.mode;
        let text = st
            .custom_text
            .lock()
            .map_or_else(|_| fresh.saver.ascii.text, |t| t.clone());
        let fps = fresh.saver.ascii.fps;
        *presenter_handle = Some(std::thread::spawn(move || {
            if crate::wayland::run_wayland_daemon_presenter(st.clone(), dwell, &mode, &text, fps)
                .is_err()
            {
                st.stop();
            }
        }));
    }
}
