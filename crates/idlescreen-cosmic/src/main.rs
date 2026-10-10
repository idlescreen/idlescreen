// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! COSMIC panel applet entry point for IdleScreen.
//!
//! Talks to `idle-daemon` over D-Bus when available, or falls back to on-disk
//! config. Mirrors idle timeout, FPS overlay, render scale, and active saver.
//! Turning the daemon on uses `systemctl --user enable --now` so it survives
//! logins; preview prefers D-Bus and falls back to `idle-daemon run-plugin`.

mod app;
mod config;
mod daemon_client;
mod i18n;

fn main() -> idle_err::Result<()> {
    idle_log::init("warn");
    cosmic::applet::run::<app::AppModel>(()).map_err(idle_err::Error::from)
}

// Applet state is owned by iced; daemon callbacks are synchronous D-Bus calls.
