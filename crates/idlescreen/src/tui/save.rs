// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Configuration persistence and single-fsync daemon reload notification for the settings TUI.

use super::model::TuiModel;
use crate::daemon::config::{Config, resolve_config_dir};
use crate::daemon::control_sock::ControlClient;

pub fn save_settings(model: &TuiModel) -> Result<String, String> {
    let cfg_dir = resolve_config_dir();
    let toml_path = cfg_dir.join("config.toml");

    let mut config = Config::load_or_migrate();

    let targets = model.active_targets();
    let active_saver = model.active_saver_name().to_string();
    config.saver.active = Some(active_saver.clone());
    config.saver.ascii.targets.clone_from(&targets);
    config.saver.ascii.mode = targets.join(",");
    config.saver.ascii.text.clone_from(&model.custom_text);
    config.saver.ascii.dwell_secs = model.dwell_secs;
    config.saver.ascii.fps = model.fps;
    config.general.idle_timeout_secs = model.idle_timeout_secs;
    config.general.dpms_timeout_secs = model.dpms_timeout_secs;

    config
        .save_to_path(&toml_path)
        .map_err(|e| format!("Failed to write {}: {e}", toml_path.display()))?;

    let mut notified = ControlClient::send_command("reload").is_ok_and(|r| r.starts_with("OK"));

    if !notified
        && let Ok(conn) = zbus::blocking::Connection::session()
        && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
    {
        let _ = proxy.set_text_targets(&targets);
        let _ = proxy.set_text(&model.custom_text);
        let _ = proxy.set_timeout(model.idle_timeout_secs);
        if proxy.set_saver(&active_saver).is_ok() {
            notified = true;
        }
    }

    let note = if notified { " (daemon updated)" } else { "" };

    Ok(format!(
        "Saved to {}{}",
        toml_path.file_name().unwrap_or_default().to_string_lossy(),
        note
    ))
}
