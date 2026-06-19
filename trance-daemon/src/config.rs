// SPDX-License-Identifier: MIT

use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub active_saver: Option<String>,
    pub idle_enabled: bool,
    pub idle_timeout_mins: u32,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            active_saver: Some("beams".to_string()),
            idle_enabled: true,
            idle_timeout_mins: 5,
        }
    }
}

impl DaemonConfig {
    fn get_config_path() -> Option<PathBuf> {
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg_config.is_empty() {
                return Some(PathBuf::from(xdg_config).join("local76").join("theme.yaml"));
            }
        }
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("local76")
                .join("theme.yaml"),
        )
    }

    pub fn load() -> Self {
        let mut config = Self::default();
        if let Some(path) = Self::get_config_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some(idx) = line.find(':') {
                        let key = line[..idx].trim();
                        let val = line[idx + 1..].trim().trim_matches('"').trim_matches('\'');
                        match key {
                            "idle_timeout_mins" => {
                                if let Ok(n) = val.parse::<u32>() {
                                    config.idle_timeout_mins = n;
                                }
                            }
                            "active_saver" => {
                                if !val.is_empty() && val != "none" {
                                    config.active_saver = Some(val.to_string());
                                } else {
                                    config.active_saver = None;
                                }
                            }
                            "idle_enabled" => {
                                if let Ok(b) = val.parse::<bool>() {
                                    config.idle_enabled = b;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        config
    }
}
