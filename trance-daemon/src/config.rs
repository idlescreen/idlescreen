// SPDX-License-Identifier: MIT

use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct DaemonConfig {
    pub active_saver: Option<String>,
    pub idle_enabled: bool,
    pub idle_timeout_mins: u32,
    pub gpu_enabled: bool,
    pub show_fps_overlay: bool,
    /// `primary`, `mirror`, or `expand` (see `trance_runner::plugin_meta::parse_display_mode`).
    pub display_mode: String,
    /// Simulation grid scale override in `(0.25, 1.0]`; `None` uses GPU/CPU defaults.
    pub render_scale: Option<f32>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            active_saver: Some("beams".to_string()),
            idle_enabled: true,
            idle_timeout_mins: 5,
            gpu_enabled: true,
            show_fps_overlay: false,
            display_mode: "primary".to_string(),
            render_scale: None,
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
                            "gpu_enabled" => {
                                if let Ok(b) = val.parse::<bool>() {
                                    config.gpu_enabled = b;
                                }
                            }
                            "show_fps_overlay" => {
                                if let Ok(b) = val.parse::<bool>() {
                                    config.show_fps_overlay = b;
                                }
                            }
                            "display_mode" => {
                                if !val.is_empty() {
                                    config.display_mode = val.to_string();
                                }
                            }
                            "render_scale" => {
                                if val.is_empty() || val.eq_ignore_ascii_case("null") {
                                    config.render_scale = None;
                                } else if let Ok(scale) = val.parse::<f32>() {
                                    config.render_scale = Some(scale.clamp(0.25, 1.0));
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

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::get_config_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let active_str = self.active_saver.as_deref().unwrap_or("none");
        let content = format!(
            "# local76 themes and settings\n\
             accent_color: \"#00BFFF\"\n\
             # dark_mode is auto-detected from system\n\
             idle_timeout_mins: {}\n\
             theme_idx: 0\n\
             active_saver: \"{}\"\n\
             idle_enabled: {}\n\
             gpu_enabled: {}\n\
             show_fps_overlay: {}\n\
             display_mode: \"{}\"\n\
             render_scale: {}\n",
            self.idle_timeout_mins,
            active_str,
            self.idle_enabled,
            self.gpu_enabled,
            self.show_fps_overlay,
            self.display_mode,
            self.render_scale
                .map(|s| s.to_string())
                .unwrap_or_else(|| "null".to_string())
        );
        fs::write(path, content)
    }
}
