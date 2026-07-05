// SPDX-License-Identifier: MIT

use std::fs;
use std::path::PathBuf;

use trance_runner::launcher::{is_allowed_saver, sanitize_saver_name};

#[derive(Debug, Clone, PartialEq)]
pub struct DaemonConfig {
    pub active_saver: Option<String>,
    pub idle_enabled: bool,
    pub idle_timeout_mins: u32,
    /// **DEPRECATED** — no-op. Retained for back-compat with existing
    /// `theme.yaml` files; the previous `trance-gpu` crate was renamed to
    /// `trance-upscaler` and is now CPU-only. See `themes.yaml(5)`.
    pub gpu_enabled: bool,
    pub show_fps_overlay: bool,
    /// Simulation grid scale override in `(0.25, 1.0]`; `None` uses CPU
    /// defaults (the GPU path was removed in 2026).
    pub render_scale: Option<f32>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            active_saver: Some("beams".to_string()),
            idle_enabled: true,
            idle_timeout_mins: 5,
            gpu_enabled: false,
            show_fps_overlay: false,
            render_scale: None,
        }
    }
}

impl DaemonConfig {
    fn get_config_path() -> Option<PathBuf> {
        let get_path_for_org = |org: &str| -> Option<PathBuf> {
            if let Some(xdg_config) = std::env::var("XDG_CONFIG_HOME")
                .ok()
                .filter(|s| !s.is_empty())
            {
                return Some(PathBuf::from(xdg_config).join(org).join("theme.yaml"));
            }
            let home = std::env::var("HOME").ok()?;
            Some(
                PathBuf::from(home)
                    .join(".config")
                    .join(org)
                    .join("theme.yaml"),
            )
        };

        let new_path = get_path_for_org("ubermetroid");
        if let Some(ref path) = new_path {
            if path.is_file() {
                return new_path;
            }
        }
        let legacy_path = get_path_for_org("local76");
        if let Some(ref path) = legacy_path {
            if path.is_file() {
                return legacy_path;
            }
        }
        new_path
    }

    pub fn load() -> Self {
        let mut config = Self::default();
        if let Some(Ok(content)) = Self::get_config_path().map(fs::read_to_string) {
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
                            if let Some(n) =
                                val.parse::<u32>().ok().filter(|&n| (1..=240).contains(&n))
                            {
                                config.idle_timeout_mins = n;
                            }
                        }
                        "active_saver" => {
                            if val.is_empty() || val == "none" {
                                config.active_saver = None;
                            } else if is_allowed_saver(val) {
                                config.active_saver =
                                    sanitize_saver_name(val).map(|s| s.to_string());
                            }
                        }
                        "idle_enabled" => {
                            if let Ok(b) = val.parse::<bool>() {
                                config.idle_enabled = b;
                            }
                        }
                        "gpu_enabled" => {
                            // DEPRECATED (2026): the previous `trance-gpu` crate
                            // was renamed to `trance-upscaler` and is now pure
                            // CPU code. `gpu_enabled` is a no-op; we accept the
                            // value silently for back-compat with existing
                            // theme.yaml files but ignore it. Logging would be
                            // spammy on every daemon start, so no warning is
                            // emitted here — the field is documented as
                            // deprecated in `themes.yaml(5)`.
                            let _ = val.parse::<bool>();
                            config.gpu_enabled = false;
                        }
                        "show_fps_overlay" => {
                            if let Ok(b) = val.parse::<bool>() {
                                config.show_fps_overlay = b;
                            }
                        }
                        "render_scale" => {
                            if val.is_empty() || val.eq_ignore_ascii_case("null") {
                                config.render_scale = None;
                            } else if let Some(scale) =
                                val.parse::<f32>().ok().filter(|s| s.is_finite())
                            {
                                config.render_scale = Some(scale.clamp(0.25, 1.0));
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
        let Some(path) = Self::get_config_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let active_str = self.active_saver.as_deref().unwrap_or("none");
        let content = format!(
            "# ubermetroid themes and settings\n\
             accent_color: \"#00BFFF\"\n\
             # dark_mode is auto-detected from system\n\
             idle_timeout_mins: {}\n\
             theme_idx: 0\n\
             active_saver: \"{}\"\n\
             idle_enabled: {}\n\
             gpu_enabled: false\n\
             show_fps_overlay: {}\n\
             render_scale: {}\n",
            self.idle_timeout_mins,
            active_str,
            self.idle_enabled,
            self.show_fps_overlay,
            self.render_scale
                .map(|s| s.to_string())
                .unwrap_or_else(|| "null".to_string())
        );
        fs::write(path, content)
    }
}
