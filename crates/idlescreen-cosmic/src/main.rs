// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! COSMIC panel applet entry point for IdleScreen.
//!
//! Talks to `idlescreen daemon` over D-Bus/CLI when available, or falls back to
//! on-disk config. Mirrors idle timeout, FPS overlay, render scale, and active
//! saver. Turning the daemon on uses `systemctl --user enable --now` so it
//! survives logins.

#![deny(unsafe_code)]

mod app;
mod config;
mod daemon_client;
mod dbus_bridge;
mod i18n;

fn main() -> dbus_bridge::Result<()> {
    cosmic::applet::run::<app::AppModel>(()).map_err(dbus_bridge::Error::from)
}
