// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! On-disk theme/settings config shared with the IdleScreen daemon.
//! Prefer `~/.config/idle/config.yaml`; fall back to legacy `~/.config/trance/`.

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
    pub show_fps_overlay: bool,
    /// `None` = auto (`null` on disk); the slider displays 1.0 for it.
    pub render_scale: Option<f32>,
}

impl ThemeConfig {
    /// Defaults used when no on-disk config is present.
    pub fn defaults() -> Self {
        Self {
            accent_color: "#00BFFF".to_string(),
            idle_timeout_mins: 5,
            theme_idx: 0,
            active_saver: Some("ascii".to_string()),
            idle_enabled: true,
            show_fps_overlay: false,
            render_scale: None,
        }
    }

    /// Write path and primary read path: `~/.config/idle/config.yaml`.
    pub fn get_config_path() -> Option<PathBuf> {
        Self::config_path_candidates().into_iter().next()
    }

    /// Idle first, legacy `trance` second (matches idle-daemon).
    pub fn config_path_candidates() -> Vec<PathBuf> {
        let mut bases = Vec::new();
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            bases.push(PathBuf::from(xdg));
        }
        if let Ok(home) = std::env::var("HOME") {
            bases.push(PathBuf::from(home).join(".config"));
        }
        let mut out = Vec::new();
        for base in bases {
            out.push(base.join("idle").join("config.yaml"));
            out.push(base.join("trance").join("config.yaml"));
        }
        out
    }

    /// Apply a single `key: value` line (YAML-ish) onto `config`.
    ///
    /// Unknown keys and malformed values are ignored so partial or hand-edited
    /// files still load.
    pub fn apply_yaml_line(config: &mut Self, line: &str) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return;
        }
        // Accept `=` too — hand-edited files commonly use it and the daemon
        // applies it, so ignoring it here would show stale defaults.
        let Some(idx) = line.find([':', '=']) else {
            return;
        };
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
            "show_fps_overlay" => {
                if let Ok(b) = val.parse::<bool>() {
                    config.show_fps_overlay = b;
                }
            }
            "render_scale" => {
                if val.is_empty() || val.eq_ignore_ascii_case("null") {
                    config.render_scale = None;
                } else if let Ok(s) = val.parse::<f32>() {
                    config.render_scale = Some(s);
                }
            }
            _ => {}
        }
    }

    /// Parse multi-line YAML-ish content into a [`ThemeConfig`].
    pub fn from_yaml_content(content: &str) -> Self {
        let mut config = Self::defaults();
        for line in content.lines() {
            Self::apply_yaml_line(&mut config, line);
        }
        config
    }

    pub fn load() -> Self {
        for path in Self::config_path_candidates() {
            if let Ok(content) = fs::read_to_string(&path) {
                return Self::from_yaml_content(&content);
            }
        }
        Self::defaults()
    }

    /// Keys this applet may write — only fields its UI can change. Every
    /// other line (`accent_color`, `theme_idx`, `theme`,
    /// `strict_control`, `[saver]` params, comments, unknowns) is foreign
    /// and passes through merge untouched.
    fn rendered_fields(&self) -> Vec<(&'static str, String)> {
        let active_str = self.active_saver.as_deref().unwrap_or("none");
        vec![
            ("idle_timeout_mins", self.idle_timeout_mins.to_string()),
            ("active_saver", format!("\"{active_str}\"")),
            ("idle_enabled", self.idle_enabled.to_string()),
            ("show_fps_overlay", self.show_fps_overlay.to_string()),
            (
                "render_scale",
                self.render_scale
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "null".to_string()),
            ),
        ]
    }

    fn field(&self, key: &str) -> Option<(&'static str, String)> {
        self.rendered_fields().into_iter().find(|(k, _)| *k == key)
    }

    /// Persist one owned key, merged into the existing file. Writing a
    /// single key means another tool's newer values — and our own
    /// possibly-stale fields — are never flattened back over the file.
    pub fn save_field(&self, key: &str) -> std::io::Result<()> {
        let Some((k, v)) = self.field(key) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("not an applet-owned config key: {key}"),
            ));
        };
        let Some(path) = Self::get_config_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Serialize read-modify-write against other writers (daemon, TUI).
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path.with_file_name("config.yaml.lock"))?;
        lock.lock()?;
        let existing = fs::read_to_string(&path).unwrap_or_default();
        let body = merge_preserving(&existing, &mut vec![(k, v)]);
        // Atomic-ish publish: tmp file + rename (matches daemon save).
        let tmp = path.with_file_name(format!("config.yaml.{}.tmp", std::process::id()));
        fs::write(&tmp, body)?;
        fs::rename(&tmp, &path).inspect_err(|_| {
            let _ = fs::remove_file(&tmp);
        })
    }
}

/// Merge owned `key: value` fields into existing config text. Foreign
/// lines — comments, unknown keys, everything under `[section]` headers —
/// pass through unchanged. `fields` is drained; leftovers append at the end.
fn merge_preserving(existing: &str, fields: &mut Vec<(&'static str, String)>) -> String {
    let mut body = String::new();
    if existing.trim().is_empty() {
        body.push_str(
            "# IdleScreen themes and settings\n# dark_mode is auto-detected from system\n",
        );
    } else {
        let mut in_section = false;
        for line in existing.lines() {
            let t = line.trim();
            if t.starts_with('[') && t.ends_with(']') {
                in_section = true;
                body.push_str(line);
                body.push('\n');
                continue;
            }
            let owned = !in_section
                && !t.is_empty()
                && !t.starts_with('#')
                && t.find([':', '=']).is_some_and(|idx| {
                    let key = t[..idx].trim();
                    if let Some(pos) = fields.iter().position(|(k, _)| *k == key) {
                        let (k, v) = fields.remove(pos);
                        body.push_str(&format!("{k}: {v}\n"));
                        true
                    } else {
                        false
                    }
                });
            if !owned {
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    for (k, v) in fields.drain(..) {
        body.push_str(&format!("{k}: {v}\n"));
    }
    body
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
