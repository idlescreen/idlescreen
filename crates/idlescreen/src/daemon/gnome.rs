// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! GNOME Mutter idle monitor fallback over D-Bus when ext-idle-notify-v1 is absent.

use zbus::proxy;

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

/// Checks whether GNOME Mutter IdleMonitor is available on the session bus.
pub fn is_mutter_idle_available(conn: &zbus::blocking::Connection) -> bool {
    MutterIdleMonitorProxyBlocking::new(conn)
        .and_then(|proxy| proxy.get_idletime())
        .is_ok()
}
