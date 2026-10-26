// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! GNOME Mutter IdleMonitor D-Bus fallback when ext-idle-notify-v1 is unavailable.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use zbus::proxy;

use super::WakeHandle;
use crate::daemon::dbus::ScreenSaverState;

#[proxy(
    interface = "org.gnome.Mutter.IdleMonitor",
    default_service = "org.gnome.Mutter.IdleMonitor",
    default_path = "/org/gnome/Mutter/IdleMonitor/Core"
)]
pub trait MutterIdleMonitor {
    fn add_idle_watch(&self, interval: u64) -> zbus::Result<u32>;
    fn add_user_active_watch(&self) -> zbus::Result<u32>;
    fn remove_watch(&self, id: u32) -> zbus::Result<()>;
    fn get_idletime(&self) -> zbus::Result<u64>;

    #[zbus(signal)]
    fn watch_fired(&self, id: u32) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.gnome.ScreenSaver",
    default_service = "org.gnome.ScreenSaver",
    default_path = "/org/gnome/ScreenSaver"
)]
pub trait GnomeScreenSaver {
    fn get_active(&self) -> zbus::Result<bool>;
    fn get_active_time(&self) -> zbus::Result<u32>;

    #[zbus(signal)]
    fn active_changed(&self, new_value: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn wake_up_screen(&self) -> zbus::Result<()>;
}

pub fn has_bus_owner(conn: &zbus::blocking::Connection, name: &str) -> bool {
    conn.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(name,),
    )
    .and_then(|r| r.body().deserialize::<bool>())
    .unwrap_or(false)
}

/// Checks whether GNOME Mutter IdleMonitor is owned and responsive on the session bus.
pub fn is_mutter_idle_available(conn: &zbus::blocking::Connection) -> bool {
    has_bus_owner(conn, "org.gnome.Mutter.IdleMonitor")
        && MutterIdleMonitorProxyBlocking::new(conn)
            .and_then(|proxy| proxy.get_idletime())
            .is_ok()
}

/// Spawns background thread monitoring GNOME Mutter IdleMonitor watch signals.
/// Re-arms the one-shot `AddUserActiveWatch` whenever idle triggers so dismissal works
/// across multiple idle cycles without polling (B1).
pub fn spawn_mutter_idle_watcher(
    state: Arc<ScreenSaverState>,
    idle_timeout_ms: u64,
    wake: WakeHandle,
) -> bool {
    let conn = zbus::blocking::connection::Builder::session()
        .ok()
        .and_then(|b| {
            b.method_timeout(std::time::Duration::from_millis(500))
                .build()
                .ok()
        });
    let Some(conn) = conn else {
        return false;
    };
    if !has_bus_owner(&conn, "org.gnome.Mutter.IdleMonitor") {
        return false;
    }
    let Ok(proxy) = MutterIdleMonitorProxyBlocking::new(&conn) else {
        return false;
    };
    let effective_ms = idle_timeout_ms.max(1000);
    let Ok(mut idle_watch_id) = proxy.add_idle_watch(effective_ms) else {
        return false;
    };
    let mut active_watch_id = proxy.add_user_active_watch().ok();
    let mut configured_ms = effective_ms;

    std::thread::Builder::new()
        .name("idle-mutter-mon".to_string())
        .spawn(move || {
            if let Ok(stream) = proxy.receive_watch_fired() {
                for signal in stream {
                    let desired_ms = state
                        .idle_timeout_secs
                        .load(Ordering::SeqCst)
                        .saturating_mul(1000)
                        .max(1000);
                    if desired_ms != configured_ms {
                        let _ = proxy.remove_watch(idle_watch_id);
                        if let Ok(new_id) = proxy.add_idle_watch(desired_ms) {
                            idle_watch_id = new_id;
                            configured_ms = desired_ms;
                        }
                    }

                    if let Ok(args) = signal.args() {
                        if args.id == idle_watch_id {
                            // Re-arm one-shot user-active watch so subsequent activity fires WatchFired!
                            active_watch_id = proxy.add_user_active_watch().ok();
                            if state.idle_enabled.load(Ordering::SeqCst)
                                && !state.session_locked.load(Ordering::SeqCst)
                                && !state.is_inhibited()
                            {
                                state.active.store(true, Ordering::SeqCst);
                                if let Ok(mut start) = state.started_at.lock() {
                                    *start = Some(Instant::now());
                                }
                                wake.wake();
                            }
                        } else if Some(args.id) == active_watch_id {
                            active_watch_id = None;
                            state.session_locked.store(false, Ordering::SeqCst);
                            if state.active.load(Ordering::SeqCst) {
                                state.stop();
                            }
                        }
                    }
                }
            }
        })
        .is_ok()
}

pub fn poll_mutter_fallback(
    mutter: &MutterIdleMonitorProxyBlocking<'_>,
    gnome_ss: Option<&GnomeScreenSaverProxyBlocking<'_>>,
    state: &Arc<ScreenSaverState>,
) {
    let is_locked = state.session_locked.load(Ordering::SeqCst)
        || gnome_ss.and_then(|g| g.get_active().ok()).unwrap_or(false);
    let idle_timeout_ms = state.idle_timeout_secs.load(Ordering::SeqCst) * 1000;
    if let Ok(t) = mutter.get_idletime() {
        if state.idle_enabled.load(Ordering::SeqCst)
            && t >= idle_timeout_ms
            && !state.is_inhibited()
            && !is_locked
        {
            state.active.store(true, Ordering::SeqCst);
        } else if (t < idle_timeout_ms || is_locked) && state.active.load(Ordering::SeqCst) {
            state.stop();
        }
    }
}
