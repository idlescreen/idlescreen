// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! TOML configuration loader, atomic saver, and migration from legacy config.yaml.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneralConfig {
    #[serde(default = "default_idle_enabled")]
    pub idle_enabled: bool,
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    #[serde(default)]
    pub dpms_timeout_secs: u64,
    #[serde(default)]
    pub primary_output: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            idle_enabled: true,
            idle_timeout_secs: 120,
            dpms_timeout_secs: 0,
            primary_output: String::new(),
        }
    }
}
fn default_idle_enabled() -> bool {
    true
}
fn default_idle_timeout_secs() -> u64 {
    120
}
fn default_mode() -> String {
    "os".to_string()
}
fn default_targets() -> Vec<String> {
    vec!["os".to_string()]
}
fn default_dwell_secs() -> f32 {
    5.0
}
fn default_dissolve_duration_secs() -> f32 {
    0.7
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AsciiSaverConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_targets")]
    pub targets: Vec<String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub fps: u32,
    #[serde(default = "default_dwell_secs")]
    pub dwell_secs: f32,
    #[serde(default = "default_dissolve_duration_secs")]
    pub dissolve_duration_secs: f32,
}

pub fn parse_targets_from_mode(mode: &str) -> Vec<String> {
    let t = mode.trim();
    if t.contains(',') {
        let parts: Vec<String> = t
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if !parts.is_empty() {
            return parts;
        }
    }
    let key = match t.to_ascii_lowercase().as_str() {
        "os" | "host" | "cycle" | "" => "os",
        "de" | "desktop" => "de",
        "kernel" | "linux" => "kernel",
        "idlescreen" | "wordmark" => "idlescreen",
        "superfedora" | "sf" => "superfedora",
        "custom" | "text" => "custom",
        other => return vec![other.to_string()],
    };
    vec![key.to_string()]
}

impl Default for AsciiSaverConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            targets: default_targets(),
            text: String::new(),
            fps: 0,
            dwell_secs: default_dwell_secs(),
            dissolve_duration_secs: default_dissolve_duration_secs(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct SaverConfig {
    #[serde(default)]
    pub ascii: AsciiSaverConfig,
    #[serde(default)]
    pub active: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub saver: SaverConfig,
}

impl Config {
    pub fn load_or_migrate() -> Self {
        let dir = resolve_config_dir();
        let (toml_p, yaml_p) = (dir.join("config.toml"), dir.join("config.yaml"));
        if toml_p.exists()
            && let Ok(s) = fs::read_to_string(&toml_p)
            && let Ok(mut cfg) = toml::from_str::<Config>(&s)
        {
            if cfg.saver.ascii.targets.is_empty()
                || cfg.saver.ascii.mode.trim().eq_ignore_ascii_case("cycle")
            {
                cfg.saver.ascii.mode = "os".to_string();
                cfg.saver.ascii.targets = vec!["os".to_string()];
            }
            return cfg;
        }
        if yaml_p.exists()
            && let Ok(cfg) = migrate_yaml_to_toml(&yaml_p, &toml_p)
        {
            return cfg;
        }
        Self::default()
    }
    pub fn save_to_path(&self, toml_path: &Path) -> std::io::Result<()> {
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        if let Some(parent) = toml_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = toml_path.with_extension(format!("tmp.{}", std::process::id()));
        {
            let mut opt = OpenOptions::new();
            let mut f = opt.write(true).create(true).truncate(true).open(&tmp)?;
            f.write_all(toml_str.as_bytes())?;
            f.flush()?;
            f.sync_all()?;
        }
        fs::rename(&tmp, toml_path)
    }

    pub fn save(&self) -> std::io::Result<()> {
        self.save_to_path(&resolve_config_dir().join("config.toml"))
    }
}

pub fn resolve_config_dir() -> PathBuf {
    if let Ok(cfg_home) = std::env::var("XDG_CONFIG_HOME")
        && !cfg_home.trim().is_empty()
    {
        return PathBuf::from(cfg_home).join("idlescreen");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config/idlescreen");
    }
    PathBuf::from("/etc/idlescreen")
}

fn strip_yaml_scalar(s: &str) -> &str {
    let s = s.trim();
    for q in ['"', '\''] {
        if let Some(inner) = s.strip_prefix(q).and_then(|r| r.split_once(q)) {
            return inner.0;
        }
    }
    s.split('#').next().unwrap_or("").trim()
}

pub fn migrate_yaml_to_toml(yaml_path: &Path, toml_path: &Path) -> std::io::Result<Config> {
    let raw = fs::read_to_string(yaml_path)?;
    let (mut cfg, mut sec, mut in_targets, mut targets) = (Config::default(), "", false, vec![]);
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if in_targets && let Some(item) = trimmed.strip_prefix('-') {
            let v = strip_yaml_scalar(item);
            if !v.is_empty() {
                targets.push(v.to_string());
            }
            continue;
        }
        in_targets = false;
        let Some((k, v_raw)) = trimmed.split_once(':') else {
            continue;
        };
        let (key, val) = (k.trim(), strip_yaml_scalar(v_raw));
        if val.is_empty() {
            match (line.len() - line.trim_start().len(), key) {
                (0, _) => sec = key,
                (_, "ascii") if matches!(sec, "saver" | "screensaver" | "ascii") => sec = "ascii",
                (_, "targets") if sec == "ascii" => in_targets = true,
                _ => {}
            }
            continue;
        }
        match (sec, key) {
            ("", "idle_timeout") | ("general", "idle_timeout_secs") => {
                cfg.general.idle_timeout_secs = val.parse().unwrap_or(120)
            }
            ("general", "idle_enabled") => cfg.general.idle_enabled = val == "true",
            ("general", "dpms_timeout_secs") => {
                cfg.general.dpms_timeout_secs = val.parse().unwrap_or(0)
            }
            ("general", "primary_output") => cfg.general.primary_output = val.to_string(),
            ("saver" | "screensaver", "active") => cfg.saver.active = Some(val.to_string()),
            ("ascii", "mode") => cfg.saver.ascii.mode = val.to_string(),
            ("ascii", "text") => cfg.saver.ascii.text = val.to_string(),
            ("ascii", "fps") => cfg.saver.ascii.fps = val.parse().unwrap_or(0),
            ("ascii", "dwell_secs") => cfg.saver.ascii.dwell_secs = val.parse().unwrap_or(5.0),
            ("ascii", "dissolve_duration_secs") => {
                cfg.saver.ascii.dissolve_duration_secs = val.parse().unwrap_or(0.7)
            }
            ("ascii", "targets") => {
                for s in val
                    .trim_matches(['[', ']'])
                    .split(',')
                    .map(strip_yaml_scalar)
                {
                    if !s.is_empty() {
                        targets.push(s.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    if targets.is_empty() {
        targets = parse_targets_from_mode(&cfg.saver.ascii.mode);
    }
    cfg.saver.ascii.targets = targets;
    let _ = cfg.save_to_path(toml_path);
    let _ = fs::rename(yaml_path, yaml_path.with_file_name("config.yaml.migrated"));
    Ok(cfg)
}
