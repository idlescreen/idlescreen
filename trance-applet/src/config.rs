// SPDX-License-Identifier: MIT

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    demo: String,
}

#[derive(Debug, Clone, Default)]
pub struct ThemeConfig {
    pub accent_color: String,
    pub idle_timeout_mins: u32,
    pub theme_idx: usize,
    pub active_saver: Option<String>,
    pub idle_enabled: bool,
    pub gpu_enabled: bool,
    pub show_fps_overlay: bool,
}

impl ThemeConfig {
    pub fn get_config_path() -> Option<PathBuf> {
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME")
            && !xdg_config.is_empty()
        {
            return Some(
                PathBuf::from(xdg_config)
                    .join("ubermetroid")
                    .join("theme.yaml"),
            );
        }
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("ubermetroid")
                .join("theme.yaml"),
        )
    }

    pub fn load() -> Self {
        let mut config = Self {
            accent_color: "#00BFFF".to_string(),
            idle_timeout_mins: 5,
            theme_idx: 0,
            active_saver: Some("beams".to_string()),
            idle_enabled: true,
            gpu_enabled: false,
            show_fps_overlay: false,
        };

        if let Some(path) = Self::get_config_path()
            && let Ok(content) = fs::read_to_string(&path)
        {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(idx) = line.find(':') {
                    let key = line[..idx].trim();
                    let val = line[idx + 1..].trim().trim_matches('"').trim_matches('\'');
                    match key {
                        "accent_color" => {
                            config.accent_color = val.to_string();
                        }
                        "idle_timeout_mins" => {
                            if let Ok(n) = val.parse::<u32>() {
                                config.idle_timeout_mins = n;
                            }
                        }
                        "theme_idx" => {
                            if let Ok(idx) = val.parse::<usize>() {
                                config.theme_idx = idx;
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
                        "gpu_enabled" => {
                            config.gpu_enabled = false;
                        }
                        "show_fps_overlay" => {
                            if let Ok(b) = val.parse::<bool>() {
                                config.show_fps_overlay = b;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        config
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = Self::get_config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let active_str = self.active_saver.as_deref().unwrap_or("none");
            let content = format!(
                "# ubermetroid themes and settings\n\
                 accent_color: \"{}\"\n\
                 # dark_mode is auto-detected from system\n\
                 idle_timeout_mins: {}\n\
                 theme_idx: {}\n\
                 active_saver: \"{}\"\n\
                 idle_enabled: {}\n\
                 gpu_enabled: false\n\
                 show_fps_overlay: {}\n",
                self.accent_color,
                self.idle_timeout_mins,
                self.theme_idx,
                active_str,
                self.idle_enabled,
                self.show_fps_overlay
            );
            fs::write(&path, content)?;
        }
        Ok(())
    }
}
