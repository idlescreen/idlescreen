// SPDX-License-Identifier: MIT

use trance_dbus::{daemon_available, DaemonStatus, TranceClient};

pub fn is_running() -> bool {
    daemon_available()
}

pub fn fetch_status() -> Option<DaemonStatus> {
    let client = TranceClient::connect().ok()?;
    client.get_status().ok()
}

pub fn set_idle_enabled(enabled: bool) -> Result<(), String> {
    let client = TranceClient::connect().map_err(|error| error.to_string())?;
    if enabled {
        client.enable().map_err(|error| error.to_string())
    } else {
        client.disable().map_err(|error| error.to_string())
    }
}

pub fn set_timeout(minutes: u32) -> Result<(), String> {
    TranceClient::connect()
        .map_err(|error| error.to_string())?
        .set_timeout(minutes)
        .map_err(|error| error.to_string())
}

pub fn set_active_saver(name: Option<&str>) -> Result<(), String> {
    TranceClient::connect()
        .map_err(|error| error.to_string())?
        .set_saver(name.unwrap_or(""))
        .map_err(|error| error.to_string())
}

pub fn set_gpu_enabled(enabled: bool) -> Result<(), String> {
    TranceClient::connect()
        .map_err(|error| error.to_string())?
        .set_gpu_enabled(enabled)
        .map_err(|error| error.to_string())
}

pub fn set_show_fps_overlay(enabled: bool) -> Result<(), String> {
    TranceClient::connect()
        .map_err(|error| error.to_string())?
        .set_show_fps_overlay(enabled)
        .map_err(|error| error.to_string())
}

pub fn set_display_mode(mode: &str) -> Result<(), String> {
    TranceClient::connect()
        .map_err(|error| error.to_string())?
        .set_display_mode(mode)
        .map_err(|error| error.to_string())
}

pub fn list_savers() -> Result<Vec<String>, String> {
    TranceClient::connect()
        .map_err(|error| error.to_string())?
        .list_savers()
        .map_err(|error| error.to_string())
}