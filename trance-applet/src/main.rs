// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 UberMetroid

//! COSMIC panel applet entry point for trance screensaver settings.
//!
//! Registers i18n, then hands off to [`app::AppModel`] which talks to the
//! trance-daemon over D-Bus when available, or falls back to on-disk config.
//! The applet mirrors daemon state for idle timeout, GPU upscale, FPS overlay,
//! display mode, and active screensaver selection.
//!
//! Build with the workspace `trance-applet` crate; requires a COSMIC/iced session.
//!
//! Systemd user units can start `trance-daemon` independently; the applet only
//! reflects and edits that service state plus local fallback configuration.

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
