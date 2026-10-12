// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Configuration persistence and daemon notification for the settings TUI.

use std::fs;

use super::model::TuiModel;
use crate::daemon::config::{Config, resolve_config_dir};
use crate::daemon::control_sock::ControlClient;

pub fn save_settings(model: &TuiModel) -> Result<String, String> {
    let cfg_dir = resolve_config_dir();
    let toml_path = cfg_dir.join("config.toml");

    let mut config = Config::load_or_migrate();

    let targets = model.active_targets();
    config.saver.ascii.targets.clone_from(&targets);
    config.saver.ascii.mode = targets.join(",");
    config.saver.ascii.text.clone_from(&model.custom_text);
    config.saver.ascii.dwell_secs = model.dwell_secs;
    config.general.idle_timeout_secs = model.idle_timeout_secs;
    config.saver.active = Some("ascii".to_string());

    let toml_str =
        toml::to_string_pretty(&config).map_err(|e| format!("Failed to serialize TOML: {e}"))?;

    fs::create_dir_all(&cfg_dir).map_err(|e| format!("Failed to create config dir: {e}"))?;

    let tmp_path = cfg_dir.join(".config.toml.tmp");
    if fs::write(&tmp_path, &toml_str).is_ok() && fs::rename(&tmp_path, &toml_path).is_ok() {
        // Atomic rename succeeded
    } else {
        let _ = fs::remove_file(&tmp_path);
        fs::write(&toml_path, toml_str)
            .map_err(|e| format!("Failed to write {}: {e}", toml_path.display()))?;
    }

    // Notify daemon if running
    let mut notified = false;
    if let Ok(conn) = zbus::blocking::Connection::session()
        && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
    {
        let _ = proxy.set_text_targets(&targets);
        let _ = proxy.set_text(&model.custom_text);
        if proxy.set_saver("ascii").is_ok() {
            notified = true;
        }
    }

    if !notified && let Ok(_resp) = ControlClient::send_command(&format!("text_targets {}", targets.join(","))) {
        let _ = ControlClient::send_command(&format!("set_text {}", model.custom_text));
        let _ = ControlClient::send_command("set ascii");
        notified = true;
    }

    let note = if notified { " (daemon updated)" } else { "" };

    Ok(format!(
        "Saved to {}{}",
        toml_path.file_name().unwrap_or_default().to_string_lossy(),
        note
    ))
}
