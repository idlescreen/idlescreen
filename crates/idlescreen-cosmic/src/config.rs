// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! On-disk theme/settings config shared with the IdleScreen daemon and TUI.
//! Primary path: `~/.config/idlescreen/config.toml`; falls back to legacy `~/.config/idle/config.yaml`.

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
    pub render_scale: Option<f32>,
}

impl ThemeConfig {
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

    pub fn get_config_path() -> Option<PathBuf> {
        Self::config_path_candidates().into_iter().next()
    }

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
        for base in &bases {
            out.push(base.join("idlescreen").join("config.toml"));
        }
        for base in &bases {
            out.push(base.join("idle").join("config.yaml"));
            out.push(base.join("trance").join("config.yaml"));
        }
        out
    }

    pub fn apply_yaml_line(config: &mut Self, line: &str) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            return;
        }
        let Some(idx) = line.find([':', '=']) else {
            return;
        };
        let key = line[..idx].trim();
        let val = line[idx + 1..].trim().trim_matches('"').trim_matches('\'');
        match key {
            "accent_color" => config.accent_color = val.to_string(),
            "idle_timeout_secs" => {
                if let Ok(secs) = val.parse::<u64>() {
                    config.idle_timeout_mins = ((secs / 60) as u32).max(1);
                }
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
            "active_saver" | "active" => {
                config.active_saver = (!val.is_empty() && val != "none").then(|| val.to_string());
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

    pub fn from_yaml_content(content: &str) -> Self {
        let mut config = Self::defaults();
        let mut ignore_section = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                ignore_section = !matches!(trimmed, "[general]" | "[saver]" | "[screensaver]");
                continue;
            }
            if !ignore_section {
                Self::apply_yaml_line(&mut config, line);
            }
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

    fn rendered_fields(&self) -> Vec<(&'static str, String)> {
        let active_str = self.active_saver.as_deref().unwrap_or("ascii");
        let timeout_secs = u64::from(self.idle_timeout_mins.max(1)) * 60;
        vec![
            ("idle_timeout_secs", timeout_secs.to_string()),
            ("active_saver", format!("\"{active_str}\"")),
            ("idle_enabled", self.idle_enabled.to_string()),
            ("show_fps_overlay", self.show_fps_overlay.to_string()),
            (
                "render_scale",
                self.render_scale
                    .map_or_else(|| "\"null\"".to_string(), |s| s.to_string()),
            ),
        ]
    }

    fn field(&self, key: &str) -> Option<(&'static str, String)> {
        let mapped = if key == "idle_timeout_mins" {
            "idle_timeout_secs"
        } else {
            key
        };
        self.rendered_fields()
            .into_iter()
            .find(|(k, _)| *k == mapped)
    }

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
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path.with_file_name("config.toml.lock"))?;
        lock.lock()?;
        let existing = fs::read_to_string(&path).unwrap_or_default();
        let body = merge_preserving(&existing, &mut vec![(k, v)]);
        let tmp = path.with_file_name(format!("config.toml.{}.tmp", std::process::id()));
        fs::write(&tmp, body)?;
        fs::rename(&tmp, &path).inspect_err(|_| {
            let _ = fs::remove_file(&tmp);
        })
    }
}

fn merge_preserving(existing: &str, fields: &mut Vec<(&'static str, String)>) -> String {
    let (mut top_lines, mut section_lines, mut sec) = (Vec::new(), Vec::new(), "");
    for line in existing.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            sec = t;
            section_lines.push(line.to_string());
            continue;
        }
        let dst = if sec.is_empty() {
            &mut top_lines
        } else {
            &mut section_lines
        };
        let editable = matches!(sec, "" | "[general]" | "[saver]" | "[screensaver]");
        let owned = editable
            && !t.is_empty()
            && !t.starts_with('#')
            && t.find([':', '=']).is_some_and(|idx| {
                let raw = t[..idx].trim();
                let key = match raw {
                    "idle_timeout_mins" => "idle_timeout_secs",
                    "active" => "active_saver",
                    other => other,
                };
                fields
                    .iter()
                    .position(|(k, _)| *k == key)
                    .is_some_and(|pos| {
                        let (k, v) = fields.remove(pos);
                        let out_k = if raw == "active" { "active" } else { k };
                        dst.push(format!("{out_k} = {v}"));
                        true
                    })
            });
        if !owned {
            dst.push(line.to_string());
        }
    }
    for (k, v) in fields.drain(..) {
        top_lines.push(format!("{k} = {v}"));
    }
    top_lines.extend(section_lines);
    let mut body = top_lines.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    body
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
