// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Wayland `ext-idle-notify-v1` idle watcher for wlroots, Sway, Hyprland, River, Niri,
//! KDE KWin, COSMIC, Labwc, Wayfire, and Weston compositors.

#![allow(unsafe_code)]

use std::os::fd::AsFd;
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1, ext_idle_notifier_v1,
};

use super::WakeHandle;
use crate::daemon::dbus::ScreenSaverState;

struct WaylandIdleState {
    daemon_state: Arc<ScreenSaverState>,
    wake: WakeHandle,
    seat: Option<wl_seat::WlSeat>,
    notifier: Option<ext_idle_notifier_v1::ExtIdleNotifierV1>,
    notification: Option<ext_idle_notification_v1::ExtIdleNotificationV1>,
    configured_timeout_secs: u64,
    configured_enabled: bool,
}

impl WaylandIdleState {
    fn sync_notification(&mut self, qh: &QueueHandle<Self>) {
        let enabled = self.daemon_state.idle_enabled.load(Ordering::SeqCst);
        let timeout_secs = self.daemon_state.idle_timeout_secs.load(Ordering::SeqCst);

        if self.notification.is_some()
            && self.configured_enabled == enabled
            && self.configured_timeout_secs == timeout_secs
        {
            return;
        }

        if let Some(old) = self.notification.take() {
            old.destroy();
        }
        self.configured_enabled = enabled;
        self.configured_timeout_secs = timeout_secs;

        if !enabled || timeout_secs == 0 {
            return;
        }

        if let (Some(notifier), Some(seat)) = (self.notifier.as_ref(), self.seat.as_ref()) {
            let timeout_ms = timeout_secs.saturating_mul(1000).min(u32::MAX as u64) as u32;
            let notif = notifier.get_idle_notification(timeout_ms, seat, qh, ());
            self.notification = Some(notif);
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandIdleState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        (): &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == "wl_seat" && state.seat.is_none() {
                state.seat =
                    Some(registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(1), qh, ()));
            } else if interface == "ext_idle_notifier_v1" && state.notifier.is_none() {
                state.notifier = Some(
                    registry.bind::<ext_idle_notifier_v1::ExtIdleNotifierV1, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    ),
                );
            }
        }
    }
}

delegate_noop!(WaylandIdleState: ignore wl_seat::WlSeat);
delegate_noop!(WaylandIdleState: ignore ext_idle_notifier_v1::ExtIdleNotifierV1);

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, ()> for WaylandIdleState {
    fn event(
        state: &mut Self,
        _proxy: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        (): &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                if state.daemon_state.idle_enabled.load(Ordering::SeqCst)
                    && !state.daemon_state.is_inhibited()
                {
                    state.daemon_state.active.store(true, Ordering::SeqCst);
                    if let Ok(mut start) = state.daemon_state.started_at.lock() {
                        *start = Some(Instant::now());
                    }
                    state.wake.wake();
                }
            }
            ext_idle_notification_v1::Event::Resumed
                if state.daemon_state.active.load(Ordering::SeqCst) =>
            {
                state.daemon_state.stop();
            }
            _ => {}
        }
    }
}

/// Probes and spawns an event-driven `ext-idle-notify-v1` Wayland idle watcher.
/// Returns `true` if the compositor supports `ext_idle_notifier_v1` and the watcher started.
pub fn spawn_wayland_idle_watcher(
    daemon_state: Arc<ScreenSaverState>,
    wake: WakeHandle,
    cfg_wake: WakeHandle,
) -> bool {
    let Ok(conn) = Connection::connect_to_env() else {
        return false;
    };
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = conn.display().get_registry(&qh, ());

    let mut wl_state = WaylandIdleState {
        daemon_state,
        wake,
        seat: None,
        notifier: None,
        notification: None,
        configured_timeout_secs: 0,
        configured_enabled: false,
    };

    if event_queue.roundtrip(&mut wl_state).is_err() {
        return false;
    }
    if wl_state.notifier.is_none() || wl_state.seat.is_none() {
        return false;
    }

    wl_state.sync_notification(&qh);
    if event_queue.flush().is_err() {
        return false;
    }

    std::thread::Builder::new()
        .name("idle-wl-mon".to_string())
        .spawn(move || {
            let cfg_fd = cfg_wake.raw_fd();
            loop {
                wl_state.sync_notification(&qh);
                if event_queue.flush().is_err() {
                    break;
                }
                if event_queue.dispatch_pending(&mut wl_state).is_err() {
                    break;
                }
                let Some(guard) = event_queue.prepare_read() else {
                    if event_queue.dispatch_pending(&mut wl_state).is_err() {
                        break;
                    }
                    continue;
                };
                let wl_fd = guard.connection_fd().as_fd().as_raw_fd();
                let mut fds = [
                    libc::pollfd {
                        fd: wl_fd,
                        events: libc::POLLIN,
                        revents: 0,
                    },
                    libc::pollfd {
                        fd: cfg_fd,
                        events: libc::POLLIN,
                        revents: 0,
                    },
                ];
                let nfds = if cfg_fd >= 0 { 2 } else { 1 };
                // SAFETY: `fds` is a valid stack array of `pollfd` descriptors.
                let poll_res = unsafe { libc::poll(fds.as_mut_ptr(), nfds, -1) };
                if poll_res < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    break;
                }
                if cfg_fd >= 0 && (fds[1].revents & libc::POLLIN) != 0 {
                    cfg_wake.drain();
                }
                if (fds[0].revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP)) != 0 {
                    if guard.read().is_err() {
                        break;
                    }
                } else {
                    drop(guard);
                }
                if event_queue.dispatch_pending(&mut wl_state).is_err() {
                    break;
                }
            }
        })
        .is_ok()
}
