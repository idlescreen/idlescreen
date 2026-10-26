// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! D-Bus service registration.
//!
//! The daemon owns three well-known bus names and serves the screensaver,
//! IdleScreen and GNOME SessionManager interfaces from one connection.

use std::sync::Arc;

use super::{IdleInterface, ScreenSaverInterface};
use crate::daemon::state::ScreenSaverState;

/// Registers the daemon's D-Bus services and claims their bus names.
pub fn register_dbus_services(state: Arc<ScreenSaverState>) -> Option<zbus::blocking::Connection> {
    use crate::daemon::watchers::session::{
        GnomeClientPrivateInterface, GnomeSessionManagerInterface,
    };
    let builder = zbus::blocking::connection::Builder::session().ok()?;
    let conn = builder
        // Bound every outgoing method call on this connection. The mutter
        // fallback runs on the event thread: a wedged compositor service must
        // not block it (and with it the daemon's SIGTERM handling).
        .method_timeout(std::time::Duration::from_millis(500))
        .name("io.github.idlescreen.Idle")
        .ok()?
        .serve_at(
            "/io/github/idlescreen/Idle",
            IdleInterface {
                state: state.clone(),
            },
        )
        .ok()?
        .serve_at(
            "/org/freedesktop/ScreenSaver",
            ScreenSaverInterface {
                state: state.clone(),
            },
        )
        .ok()?
        .serve_at(
            "/org/gnome/SessionManager",
            GnomeSessionManagerInterface { state },
        )
        .ok()?
        .serve_at(
            "/org/gnome/SessionManager/Client1",
            GnomeClientPrivateInterface,
        )
        .ok()?
        .build()
        .ok()?;
    let _ = conn.request_name("org.freedesktop.ScreenSaver");
    let _ = conn.request_name("org.gnome.SessionManager");
    Some(conn)
}
