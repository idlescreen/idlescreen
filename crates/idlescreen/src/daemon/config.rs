// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! TOML configuration loader and atomic migration from legacy config.yaml.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneralConfig {
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    #[serde(default)]
    pub primary_output: String,
}

fn default_idle_timeout_secs() -> u64 {
    120
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: default_idle_timeout_secs(),
            primary_output: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AsciiSaverConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub text: String,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_dwell_secs")]
    pub dwell_secs: f32,
    #[serde(default = "default_dissolve_duration_secs")]
    pub dissolve_duration_secs: f32,
}

fn default_mode() -> String {
    "cycle".to_string()
}
fn default_fps() -> u32 {
    60
}
fn default_dwell_secs() -> f32 {
    5.0
}
fn default_dissolve_duration_secs() -> f32 {
    0.7
}

impl Default for AsciiSaverConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            text: String::new(),
            fps: default_fps(),
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
    /// Loads configuration from standard path or performs deterministic migration.
    pub fn load_or_migrate() -> Self {
        let config_dir = resolve_config_dir();
        let toml_path = config_dir.join("config.toml");
        let yaml_path = config_dir.join("config.yaml");

        if toml_path.exists()
            && let Ok(content) = fs::read_to_string(&toml_path)
            && let Ok(cfg) = toml::from_str::<Config>(&content)
        {
            return cfg;
        }

        if yaml_path.exists()
            && let Ok(cfg) = migrate_yaml_to_toml(&yaml_path, &toml_path)
        {
            return cfg;
        }

        Self::default()
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

/// Migrates a legacy YAML configuration file to TOML atomically.
pub fn migrate_yaml_to_toml(yaml_path: &Path, toml_path: &Path) -> std::io::Result<Config> {
    // Resolve symlink target if GNU Stow or dotfile symlink is used
    let real_yaml_path = fs::canonicalize(yaml_path)?;
    let yaml_content = fs::read_to_string(&real_yaml_path)?;

    // Parse legacy YAML
    let yaml_val: serde_yaml::Value = serde_yaml::from_str(&yaml_content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    let mut config = Config::default();

    if let Some(general) = yaml_val.get("general") {
        if let Some(timeout) = general
            .get("idle_timeout_secs")
            .and_then(serde_yaml::Value::as_u64)
        {
            config.general.idle_timeout_secs = timeout;
        }
        if let Some(primary) = general.get("primary_output").and_then(|v| v.as_str()) {
            config.general.primary_output = primary.to_string();
        }
    } else if let Some(timeout) = yaml_val
        .get("idle_timeout")
        .and_then(serde_yaml::Value::as_u64)
    {
        config.general.idle_timeout_secs = timeout;
    }

    if let Some(saver) = yaml_val
        .get("saver")
        .or_else(|| yaml_val.get("screensaver"))
    {
        if let Some(active) = saver.get("active").and_then(|v| v.as_str()) {
            config.saver.active = Some(active.to_string());
        }
        if let Some(ascii) = saver.get("ascii") {
            if let Some(mode) = ascii.get("mode").and_then(|v| v.as_str()) {
                config.saver.ascii.mode = mode.to_string();
            }
            if let Some(text) = ascii.get("text").and_then(|v| v.as_str()) {
                config.saver.ascii.text = text.to_string();
            }
            if let Some(fps) = ascii.get("fps").and_then(serde_yaml::Value::as_u64) {
                config.saver.ascii.fps = fps as u32;
            }
            if let Some(dwell) = ascii.get("dwell_secs").and_then(serde_yaml::Value::as_f64) {
                config.saver.ascii.dwell_secs = dwell as f32;
            }
            if let Some(dissolve) = ascii
                .get("dissolve_duration_secs")
                .and_then(serde_yaml::Value::as_f64)
            {
                config.saver.ascii.dissolve_duration_secs = dissolve as f32;
            }
        }
    }

    // Write toml atomically
    let toml_string = toml::to_string_pretty(&config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    if let Some(parent) = toml_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let pid = std::process::id();
    let tmp_path = toml_path.with_extension(format!("tmp.{pid}"));
    {
        let mut tmp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;
        tmp_file.write_all(toml_string.as_bytes())?;
        tmp_file.flush()?;
        tmp_file.sync_all()?;
    }

    fs::rename(&tmp_path, toml_path)?;

    // Rename legacy yaml to config.yaml.migrated
    let migrated_path = real_yaml_path.with_file_name("config.yaml.migrated");
    let _ = fs::rename(&real_yaml_path, &migrated_path);

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_to_toml_migration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let yaml_path = temp_dir.path().join("config.yaml");
        let toml_path = temp_dir.path().join("config.toml");

        let yaml_src = r#"
general:
  idle_timeout_secs: 180
  primary_output: "DP-1"
saver:
  active: "aurora"
  ascii:
    mode: "os"
    text: "MyFedora"
    fps: 30
"#;
        fs::write(&yaml_path, yaml_src).unwrap();

        let cfg = migrate_yaml_to_toml(&yaml_path, &toml_path).expect("migration must succeed");
        assert_eq!(cfg.general.idle_timeout_secs, 180);
        assert_eq!(cfg.general.primary_output, "DP-1");
        assert_eq!(cfg.saver.active.as_deref(), Some("aurora"));
        assert_eq!(cfg.saver.ascii.mode, "os");
        assert_eq!(cfg.saver.ascii.text, "MyFedora");
        assert_eq!(cfg.saver.ascii.fps, 30);

        assert!(toml_path.exists());
        assert!(temp_dir.path().join("config.yaml.migrated").exists());
        assert!(!yaml_path.exists());
    }
}
