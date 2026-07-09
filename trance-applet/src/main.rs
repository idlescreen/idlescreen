// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 UberMetroid

//! COSMIC panel applet entry point for trance screensaver settings.
//!
//! Talks to `trance-daemon` over D-Bus when available, or falls back to on-disk
//! config. Mirrors idle timeout, FPS overlay, render scale, and active saver.
//! Turning the daemon on uses `systemctl --user enable --now` so it survives
//! logins; preview prefers D-Bus and falls back to `trance-daemon run-plugin`.

mod app;
mod config;
mod daemon_client;
mod i18n;

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .try_init();
}

fn main() -> anyhow::Result<()> {
    init_tracing();
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);
    cosmic::applet::run::<app::AppModel>(()).map_err(anyhow::Error::from)
}

// Applet state is owned by iced; daemon callbacks are synchronous D-Bus calls.
