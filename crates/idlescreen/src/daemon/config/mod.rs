// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Zero-dependency TOML configuration loader, atomic saver, and YAML migration.

use std::fmt::Write as FmtWrite;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

mod parse_toml;

pub use parse_toml::migrate_yaml_to_toml;
use parse_toml::quote_toml_str;

#[derive(Clone, Debug, PartialEq)]
pub struct GeneralConfig {
    pub idle_enabled: bool,
    pub idle_timeout_secs: u64,
    pub dpms_timeout_secs: u64,
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

#[derive(Clone, Debug, PartialEq)]
pub struct AsciiSaverConfig {
    pub mode: String,
    pub targets: Vec<String>,
    pub text: String,
    pub fps: u32,
    pub dwell_secs: f32,
    pub dissolve_duration_secs: f32,
}

impl Default for AsciiSaverConfig {
    fn default() -> Self {
        Self {
            mode: "os".to_string(),
            targets: vec!["os".to_string()],
            text: String::new(),
            fps: 0,
            dwell_secs: 5.0,
            dissolve_duration_secs: 0.7,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SaverConfig {
    pub ascii: AsciiSaverConfig,
    pub active: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub saver: SaverConfig,
}

pub fn parse_targets_from_mode(mode: &str) -> Vec<String> {
    let t = mode.trim();
    if t.contains(',') {
        let p: Vec<String> = t
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if !p.is_empty() {
            return p;
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

impl Config {
    pub fn load_or_migrate() -> Self {
        let dir = resolve_config_dir();
        let (toml_p, yaml_p) = (dir.join("config.toml"), dir.join("config.yaml"));
        if toml_p.exists()
            && let Ok(s) = fs::read_to_string(&toml_p)
            && let Ok(mut cfg) = Self::from_toml_str(&s)
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

    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        parse_toml::parse_toml_config(s)
    }

    pub fn to_toml_string(&self) -> String {
        let mut o = String::with_capacity(512);
        let (g, a) = (&self.general, &self.saver.ascii);
        let _ = writeln!(
            o,
            "[general]\nidle_enabled = {}\nidle_timeout_secs = {}\ndpms_timeout_secs = {}\nprimary_output = {}\n\n[saver]",
            g.idle_enabled,
            g.idle_timeout_secs,
            g.dpms_timeout_secs,
            quote_toml_str(&g.primary_output)
        );
        if let Some(ref act) = self.saver.active {
            let _ = writeln!(o, "active = {}", quote_toml_str(act));
        }
        let tgts: Vec<String> = a.targets.iter().map(|s| quote_toml_str(s)).collect();
        let _ = writeln!(
            o,
            "\n[saver.ascii]\nmode = {}\ntargets = [{}]\ntext = {}\nfps = {}\ndwell_secs = {:?}\ndissolve_duration_secs = {:?}",
            quote_toml_str(&a.mode),
            tgts.join(", "),
            quote_toml_str(&a.text),
            a.fps,
            a.dwell_secs,
            a.dissolve_duration_secs
        );
        o
    }

    pub fn save_to_path(&self, toml_path: &Path) -> std::io::Result<()> {
        if let Some(parent) = toml_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = toml_path.with_extension(format!("tmp.{}", std::process::id()));
        {
            let mut f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp)?;
            f.write_all(self.to_toml_string().as_bytes())?;
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
    if let Ok(c) = std::env::var("XDG_CONFIG_HOME")
        && !c.trim().is_empty()
    {
        return PathBuf::from(c).join("idlescreen");
    }
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h).join(".config/idlescreen");
    }
    PathBuf::from("/etc/idlescreen")
}
