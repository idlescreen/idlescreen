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
    /// `config.yaml` files; the previous `trance-gpu` crate was renamed to
    /// `trance-upscaler` and is now CPU-only. See `config.yaml(5)`.
    #[deprecated(
        note = "GPU upscaler removed in 2026; field retained for back-compat, will be removed in 0.4"
    )]
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
    pub fn get_config_path() -> Option<PathBuf> {
        if let Some(xdg_config) = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|s| !s.is_empty())
        {
            return Some(PathBuf::from(xdg_config).join("trance").join("config.yaml"));
        }
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("trance")
                .join("config.yaml"),
        )
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
                            // config.yaml files but ignore it. Logging would be
                            // spammy on every daemon start, so no warning is
                            // emitted here — the field is documented as
                            // deprecated in `config.yaml(5)`.
                            let _ = val.parse::<bool>();
                            #[allow(deprecated)]
                            {
                                config.gpu_enabled = false;
                            }
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
        let parent = path
            .parent()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no parent dir"))?;
        fs::create_dir_all(parent)?;
        let active_str = self.active_saver.as_deref().unwrap_or("none");
        let content = format!(
            "# trance themes and settings\n\
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
        static TMP_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp_path = parent.join(format!("config.tmp.{}.{}", std::process::id(), count));
        fs::write(&tmp_path, content)?;
        fs::rename(tmp_path, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_5_minute_timeout() {
        let c = DaemonConfig::default();
        assert_eq!(c.idle_timeout_mins, 5);
    }

    #[test]
    fn default_saver_is_beams() {
        let c = DaemonConfig::default();
        assert_eq!(c.active_saver.as_deref(), Some("beams"));
    }

    #[test]
    fn default_idle_enabled() {
        let c = DaemonConfig::default();
        assert!(c.idle_enabled);
    }

    #[test]
    fn default_render_scale_is_none() {
        let c = DaemonConfig::default();
        assert!(c.render_scale.is_none());
    }

    #[test]
    fn default_show_fps_overlay_false() {
        let c = DaemonConfig::default();
        assert!(!c.show_fps_overlay);
    }
}
