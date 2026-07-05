// SPDX-License-Identifier: MIT

use anyhow::{Context, Result};
use trance_dbus::{DaemonStatus, TranceClient, daemon_available};

pub fn is_running() -> bool {
    daemon_available()
}

#[tracing::instrument]
pub fn fetch_status() -> Result<DaemonStatus> {
    let client = TranceClient::connect().context("failed to connect to trance daemon")?;
    client.get_status().context("failed to fetch daemon status")
}

pub fn set_idle_enabled(enabled: bool) -> Result<()> {
    let client = TranceClient::connect().context("failed to connect to trance daemon")?;
    if enabled {
        client.enable().context("failed to enable idle activation")
    } else {
        client
            .disable()
            .context("failed to disable idle activation")
    }
}

pub fn set_timeout(minutes: u32) -> Result<()> {
    TranceClient::connect()
        .context("failed to connect to trance daemon")?
        .set_timeout(minutes)
        .context("failed to set idle timeout")
}

pub fn set_active_saver(name: Option<&str>) -> Result<()> {
    TranceClient::connect()
        .context("failed to connect to trance daemon")?
        .set_saver(name.unwrap_or(""))
        .context("failed to set active screensaver")
}

pub fn set_show_fps_overlay(enabled: bool) -> Result<()> {
    TranceClient::connect()
        .context("failed to connect to trance daemon")?
        .set_show_fps_overlay(enabled)
        .context("failed to set FPS overlay")
}

#[tracing::instrument]
pub fn list_savers() -> Result<Vec<String>> {
    TranceClient::connect()
        .context("failed to connect to trance daemon")?
        .list_savers()
        .context("failed to list installed screensavers")
}

#[tracing::instrument]
pub fn start_preview(name: &str) -> Result<()> {
    TranceClient::connect()
        .context("failed to connect to trance daemon")?
        .preview(name)
        .context("failed to request screensaver preview")
}
