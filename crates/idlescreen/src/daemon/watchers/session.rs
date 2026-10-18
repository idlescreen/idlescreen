// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Consolidated session/system bus watchers for lock, unlock, suspend/resume,
//! and D-Bus inhibitor client disconnect cleanup.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use zbus::{interface, proxy};

use super::WakeHandle;
use super::mutter::{GnomeScreenSaverProxyBlocking, has_bus_owner};
use crate::daemon::dbus::{InhibitEntry, ScreenSaverState};

pub struct GnomeSessionManagerInterface {
    pub state: Arc<ScreenSaverState>,
}

#[interface(name = "org.gnome.SessionManager")]
#[allow(unused_variables, clippy::needless_pass_by_value)]
impl GnomeSessionManagerInterface {
    fn inhibit(
        &self,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        app_id: String,
        toplevel_xid: u32,
        reason: String,
        flags: u32,
    ) -> u32 {
        let cookie = self.state.cookie_counter.fetch_add(1, Ordering::SeqCst);
        if flags == 0 || (flags & 8) != 0 {
            let sender = hdr.sender().map(std::string::ToString::to_string);
            let app_name = if app_id.trim().is_empty() {
                "portal-inhibit".to_string()
            } else {
                app_id
            };
            let reason = if reason.trim().is_empty() {
                "Idle inhibited".to_string()
            } else {
                reason
            };
            if let Ok(mut guard) = self.state.inhibitors.lock() {
                guard.push(InhibitEntry {
                    cookie,
                    app_name,
                    reason,
                    sender,
                });
            }
            if self.state.active.load(Ordering::SeqCst) {
                self.state.stop();
            } else {
                self.state.wake();
            }
        }
        cookie
    }

    fn uninhibit(&self, inhibit_cookie: u32) {
        if let Ok(mut guard) = self.state.inhibitors.lock() {
            guard.retain(|e| e.cookie != inhibit_cookie);
        }
        self.state.wake_idle_cfg();
        self.state.wake();
    }

    fn is_inhibited(&self, flags: u32) -> bool {
        (flags == 0 || (flags & 8) != 0) && self.state.is_inhibited()
    }

    #[rustfmt::skip]
    fn get_inhibitors(&self) -> Vec<zbus::zvariant::OwnedObjectPath> { Vec::new() }
    #[rustfmt::skip]
    fn get_clients(&self) -> Vec<zbus::zvariant::OwnedObjectPath> { Vec::new() }
    #[rustfmt::skip]
    fn register_client(&self, app_id: &str, client_startup_id: &str) -> zbus::zvariant::OwnedObjectPath {
        zbus::zvariant::ObjectPath::from_static_str_unchecked("/org/gnome/SessionManager/Client1").into()
    }
    fn unregister_client(&self, client_id: zbus::zvariant::ObjectPath<'_>) {}
    #[rustfmt::skip]
    fn is_session_running(&self) -> bool { true }
    #[zbus(property)]
    #[rustfmt::skip]
    fn inhibited_actions(&self) -> u32 { if self.state.is_inhibited() { 8 } else { 0 } }
    #[zbus(property)]
    #[rustfmt::skip]
    fn session_is_active(&self) -> bool { true }
    #[zbus(property)]
    #[rustfmt::skip]
    fn session_name(&self) -> String { "idlescreen".to_string() }
}

pub struct GnomeClientPrivateInterface;

#[interface(name = "org.gnome.SessionManager.ClientPrivate")]
#[allow(unused_variables)]
impl GnomeClientPrivateInterface {
    fn end_session_response(&self, is_ok: bool, reason: &str) {}
}

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

    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;
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
            let add_match = |rule: &str| {
                let _ = conn.call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "AddMatch",
                    &(rule,),
                );
            };
            add_match("type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',member='NameOwnerChanged'");
            if has_bus_owner(&conn, "org.gnome.ScreenSaver") {
                add_match("type='signal',interface='org.gnome.ScreenSaver'");
            }

            for msg in zbus::blocking::MessageIterator::from(&conn).flatten() {
                let hdr = msg.header();
                let Some(member) = hdr.member().map(zbus::names::MemberName::as_str) else {
                    continue;
                };
                let iface = hdr.interface().map(zbus::names::InterfaceName::as_str);

                if iface == Some("org.gnome.ScreenSaver")
                    && (member == "ActiveChanged" || member == "WakeUpScreen")
                {
                    if member == "ActiveChanged"
                        && let Ok(locked) = msg.body().deserialize::<bool>()
                    {
                        st_session.session_locked.store(locked, Ordering::SeqCst);
                        st_session.wake_idle_cfg();
                    }
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
            if let Ok(sess) = LogindSessionProxyBlocking::new(&conn)
                && let Ok(locked) = sess.locked_hint()
            {
                st_sys.session_locked.store(locked, Ordering::SeqCst);
            }
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
            for msg in zbus::blocking::MessageIterator::from(&conn).flatten() {
                if let Some(member) = msg.header().member().map(zbus::names::MemberName::as_str)
                    && matches!(member, "PrepareForSleep" | "Lock" | "Unlock")
                {
                    st_sys
                        .session_locked
                        .store(member == "Lock", Ordering::SeqCst);
                    st_sys.wake_idle_cfg();
                    st_sys.stop();
                    wake_sys.wake();
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_locked_state_blocks_and_clears() {
        let state = ScreenSaverState::new();
        assert!(!state.session_locked.load(Ordering::SeqCst));
        state.session_locked.store(true, Ordering::SeqCst);
        assert!(state.session_locked.load(Ordering::SeqCst));
        state.session_locked.store(false, Ordering::SeqCst);
        assert!(!state.session_locked.load(Ordering::SeqCst));
    }
}
