// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use std::fs;
use std::path::PathBuf;

use crate::config_parse::apply_config_line;

#[derive(Debug, Clone, PartialEq)]
pub struct DaemonConfig {
    pub active_saver: Option<String>,
    pub idle_enabled: bool,
    pub idle_timeout_mins: u32,
    /// **DEPRECATED** — no-op. Retained for back-compat with existing
    /// `config.yaml` files; the previous `trance-gpu` crate was renamed to
    /// `idle-upscaler` and is now CPU-only. See `config.yaml(5)`.
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

/// Absolute path without `..` or NUL — blocks env-based config path traversal.
fn is_safe_config_root(path: &str) -> bool {
    if path.is_empty() || path.contains('\0') {
        return false;
    }
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return false;
    }
    !p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
}

impl DaemonConfig {
    /// Config directory candidates: IdleScreen first, legacy `trance` second.
    pub fn config_dir_candidates() -> Vec<PathBuf> {
        let mut bases = Vec::new();
        if let Some(xdg) = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|s| is_safe_config_root(s))
        {
            bases.push(PathBuf::from(xdg));
        }
        if let Ok(home) = std::env::var("HOME")
            && is_safe_config_root(&home)
        {
            bases.push(PathBuf::from(home).join(".config"));
        }
        let mut dirs = Vec::new();
        for base in bases {
            dirs.push(base.join("idle"));
            dirs.push(base.join("trance"));
        }
        dirs
    }

    /// Path used for **writes** and new installs (`~/.config/idle/config.yaml`).
    pub fn get_config_path() -> Option<PathBuf> {
        Self::config_dir_candidates()
            .into_iter()
            .find(|d| d.ends_with("idle"))
            .map(|d| d.join("config.yaml"))
    }

    /// Resolve existing config for **reads**: prefer IdleScreen, fall back to legacy.
    pub fn resolve_config_path() -> Option<PathBuf> {
        let candidates: Vec<PathBuf> = Self::config_dir_candidates()
            .into_iter()
            .map(|d| d.join("config.yaml"))
            .collect();
        candidates
            .iter()
            .find(|p| p.is_file())
            .cloned()
            .or_else(|| candidates.into_iter().next())
    }

    pub fn load() -> Self {
        let mut config = Self::default();
        if let Some(Ok(content)) = Self::resolve_config_path().map(fs::read_to_string) {
            for line in content.lines() {
                apply_config_line(&mut config, line);
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
    fn safe_config_root_rejects_traversal() {
        assert!(is_safe_config_root("/home/user/.config"));
        assert!(!is_safe_config_root(""));
        assert!(!is_safe_config_root("relative"));
        assert!(!is_safe_config_root("/home/user/../etc"));
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
