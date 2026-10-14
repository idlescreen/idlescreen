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
    pub primary_output: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            idle_enabled: true,
            idle_timeout_secs: 120,
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
        let config_dir = resolve_config_dir();
        let toml_path = config_dir.join("config.toml");
        let yaml_path = config_dir.join("config.yaml");

        if toml_path.exists()
            && let Ok(content) = fs::read_to_string(&toml_path)
            && let Ok(mut cfg) = toml::from_str::<Config>(&content)
        {
            if cfg.saver.ascii.targets.is_empty()
                || cfg.saver.ascii.mode.trim().eq_ignore_ascii_case("cycle")
                || cfg.saver.ascii.targets == ["os", "de"]
            {
                cfg.saver.ascii.mode = "os".to_string();
                cfg.saver.ascii.targets = vec!["os".to_string()];
            }
            return cfg;
        }
        if yaml_path.exists()
            && let Ok(cfg) = migrate_yaml_to_toml(&yaml_path, &toml_path)
        {
            return cfg;
        }
        Self::default()
    }

    pub fn save_to_path(&self, toml_path: &Path) -> std::io::Result<()> {
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        if let Some(parent) = toml_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = toml_path.with_extension(format!("tmp.{}", std::process::id()));
        {
            let mut f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)?;
            f.write_all(toml_string.as_bytes())?;
            f.flush()?;
            f.sync_all()?;
        }
        fs::rename(&tmp_path, toml_path)
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

pub fn migrate_yaml_to_toml(yaml_path: &Path, toml_path: &Path) -> std::io::Result<Config> {
    let yaml_val: serde_yaml::Value = serde_yaml::from_str(&fs::read_to_string(yaml_path)?)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    let mut config = Config::default();
    if let Some(g) = yaml_val.get("general") {
        if let Some(v) = g.get("idle_enabled").and_then(serde_yaml::Value::as_bool) {
            config.general.idle_enabled = v;
        }
        if let Some(v) = g
            .get("idle_timeout_secs")
            .and_then(serde_yaml::Value::as_u64)
        {
            config.general.idle_timeout_secs = v;
        }
        if let Some(v) = g.get("primary_output").and_then(|v| v.as_str()) {
            config.general.primary_output = v.to_string();
        }
    } else if let Some(v) = yaml_val
        .get("idle_timeout")
        .and_then(serde_yaml::Value::as_u64)
    {
        config.general.idle_timeout_secs = v;
    }

    if let Some(saver) = yaml_val
        .get("saver")
        .or_else(|| yaml_val.get("screensaver"))
    {
        if let Some(active) = saver.get("active").and_then(|v| v.as_str()) {
            config.saver.active = Some(active.to_string());
        }
        if let Some(ascii) = saver.get("ascii") {
            let a = &mut config.saver.ascii;
            if let Some(v) = ascii.get("mode").and_then(|v| v.as_str()) {
                a.mode = v.to_string();
            }
            if let Some(v) = ascii.get("text").and_then(|v| v.as_str()) {
                a.text = v.to_string();
            }
            if let Some(v) = ascii.get("fps").and_then(serde_yaml::Value::as_u64) {
                a.fps = u32::try_from(v).unwrap_or(0);
            }
            if let Some(v) = ascii.get("dwell_secs").and_then(serde_yaml::Value::as_f64) {
                a.dwell_secs = v as f32;
            }
            if let Some(v) = ascii
                .get("dissolve_duration_secs")
                .and_then(serde_yaml::Value::as_f64)
            {
                a.dissolve_duration_secs = v as f32;
            }
            let seq = ascii.get("targets").and_then(|v| v.as_sequence());
            a.targets = seq.map_or_else(
                || parse_targets_from_mode(&a.mode),
                |s| {
                    s.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                },
            );
        }
    }
    let _ = config.save_to_path(toml_path);
    let _ = fs::rename(yaml_path, yaml_path.with_file_name("config.yaml.migrated"));
    Ok(config)
}
