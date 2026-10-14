// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Consolidated session/system bus watchers for lock, unlock, suspend/resume,
//! and D-Bus inhibitor client disconnect cleanup.

use std::sync::Arc;
use zbus::proxy;

use super::WakeHandle;
use super::mutter::{GnomeScreenSaverProxyBlocking, has_bus_owner};
use crate::daemon::dbus::ScreenSaverState;

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
pub trait LogindManager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
pub trait LogindSession {
    #[zbus(signal)]
    fn lock(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unlock(&self) -> zbus::Result<()>;
}

/// Spawns consolidated session and system D-Bus watchers (systemd-logind, elogind,
/// GNOME ScreenSaver, and D-Bus NameOwnerChanged for orphaned inhibitor cleanup).
pub fn spawn_gnome_session_watchers(state: Arc<ScreenSaverState>, wake: WakeHandle) {
    // 1. Session bus watcher: NameOwnerChanged (inhibitor cleanup) + optional GNOME ScreenSaver
    let st_session = state.clone();
    let wake_session = wake.clone();
    std::thread::Builder::new()
        .name("idle-sess-mon".to_string())
        .spawn(move || {
            let Ok(conn) = zbus::blocking::Connection::session() else {
                return;
            };

            // Add match rules for NameOwnerChanged and (if on GNOME) org.gnome.ScreenSaver
            let _ = conn.call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "AddMatch",
                &("type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',member='NameOwnerChanged'",),
            );
            if has_bus_owner(&conn, "org.gnome.ScreenSaver") {
                let _ = conn.call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "AddMatch",
                    &("type='signal',interface='org.gnome.ScreenSaver'",),
                );
            }

            let iter = zbus::blocking::MessageIterator::from(&conn);
            for msg in iter.flatten() {
                let hdr = msg.header();
                let Some(member) = hdr.member().map(zbus::names::MemberName::as_str) else {
                    continue;
                };
                let iface = hdr.interface().map(zbus::names::InterfaceName::as_str);

                if iface == Some("org.gnome.ScreenSaver")
                    && (member == "ActiveChanged" || member == "WakeUpScreen")
                {
                    st_session.stop();
                    wake_session.wake();
                } else if iface == Some("org.freedesktop.DBus")
                    && member == "NameOwnerChanged"
                    && let Ok((name, _old_owner, new_owner)) =
                        msg.body().deserialize::<(&str, &str, &str)>()
                    && new_owner.is_empty()
                {
                    st_session.remove_inhibitors_by_sender(name);
                }
            }
        })
        .ok();

    // 2. Consolidated system bus watcher: systemd-logind / elogind / ConsoleKit Lock, Unlock, PrepareForSleep
    let st_sys = state;
    let wake_sys = wake;
    std::thread::Builder::new()
        .name("idle-sys-mon".to_string())
        .spawn(move || {
            let Ok(conn) = zbus::blocking::Connection::system() else {
                return;
            };
            for rule in [
                "type='signal',interface='org.freedesktop.login1.Manager',member='PrepareForSleep'",
                "type='signal',interface='org.freedesktop.login1.Session',member='Lock'",
                "type='signal',interface='org.freedesktop.login1.Session',member='Unlock'",
                "type='signal',interface='org.freedesktop.ConsoleKit.Session',member='Lock'",
                "type='signal',interface='org.freedesktop.ConsoleKit.Session',member='Unlock'",
            ] {
                let _ = conn.call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "AddMatch",
                    &(rule,),
                );
            }

            let _ = GnomeScreenSaverProxyBlocking::builder(&conn);
            let iter = zbus::blocking::MessageIterator::from(&conn);
            for msg in iter.flatten() {
                let hdr = msg.header();
                if let Some(member) = hdr.member().map(zbus::names::MemberName::as_str)
                    && matches!(member, "PrepareForSleep" | "Lock" | "Unlock")
                {
                    st_sys.stop();
                    wake_sys.wake();
                }
            }
        })
        .ok();
}
