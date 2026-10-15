// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Target mode choices, FPS presets, and focusable field definitions for the settings TUI.

use crate::ascii::effects::EffectKind;
use crate::daemon::config::Config;

pub const FPS_PRESETS: [u32; 5] = [0, 30, 60, 120, 144];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeChoice {
    HostOs,
    CustomText,
}

impl ModeChoice {
    pub const ALL: [ModeChoice; 2] = [ModeChoice::HostOs, ModeChoice::CustomText];

    pub fn to_config_str(self) -> &'static str {
        match self {
            ModeChoice::HostOs => "os",
            ModeChoice::CustomText => "custom",
        }
    }

    pub fn from_config_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "custom" | "text" => ModeChoice::CustomText,
            _ => ModeChoice::HostOs,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ModeChoice::HostOs => "OS Name (Auto)",
            ModeChoice::CustomText => "Custom Banner Text",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuiField {
    ActiveSaver,       // 0
    ModeChoice(usize), // 1..=2
    CustomText,        // 3
    IdleTimeout,       // 4
    DwellDuration,     // 5
    FrameRate,         // 6
    SaveButton,        // 7
    PreviewButton,     // 8
    FullscreenButton,  // 9
    QuitButton,        // 10
}

pub const TOTAL_FIELDS: usize = 11;

pub fn build_available_savers() -> Vec<String> {
    let mut savers = Vec::with_capacity(53);
    if crate::ascii::sys_info::is_superfedora_machine() {
        savers.push("superfedora".to_string());
    }
    savers.push("ascii".to_string());
    savers.push("tour".to_string());
    for kind in EffectKind::ALL {
        savers.push(kind.name().to_string());
    }
    savers
}

pub fn resolve_initial_mode_and_text(config: &Config, detected_os: &str) -> (ModeChoice, String) {
    let mut custom_text = config.saver.ascii.text.clone();
    let raw_items: Vec<String> = if config.saver.ascii.targets.is_empty() {
        config
            .saver
            .ascii
            .mode
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    } else {
        config.saver.ascii.targets.clone()
    };

    let mut mode = ModeChoice::HostOs;
    for item in &raw_items {
        match item.trim().to_ascii_lowercase().as_str() {
            "custom" | "text" => mode = ModeChoice::CustomText,
            "os" | "host" | "superfedora" | "sf" | "cycle" | "de" | "desktop" | "kernel"
            | "linux" | "idlescreen" | "wordmark" | "" => {}
            other if !other.eq_ignore_ascii_case(detected_os) => {
                mode = ModeChoice::CustomText;
                if custom_text.is_empty() {
                    custom_text = other.to_string();
                }
            }
            _ => {}
        }
    }
    (mode, custom_text)
}

pub fn format_saver_display(name: &str) -> String {
    match name {
        "superfedora" => "superfedora (16-Bit SuperFedora Arcade)".to_string(),
        "ascii" => "ascii (All 50 Arcade Scenes Shuffled)".to_string(),
        "tour" => "tour (All 50 Arcade Scenes in Order 1-50)".to_string(),
        scene => {
            if EffectKind::NEW_SCENES.iter().any(|k| k.name() == scene) {
                format!("{scene} (New 16-Bit Arcade Scene)")
            } else {
                format!("{scene} (Pinned Arcade Scene)")
            }
        }
    }
}

pub fn format_fps_display(fps: u32) -> String {
    if fps == 0 {
        "Auto (Monitor Hz)".to_string()
    } else {
        format!("{fps} FPS")
    }
}

pub fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map_or(s.len(), |(idx, _)| idx)
}

pub fn format_inhibitor_summary(raw_json: &str) -> String {
    let trimmed = raw_json.trim();
    if trimmed == "[]" || trimmed.is_empty() {
        return "Uninhibited".to_string();
    }
    let count = trimmed.matches("\"cookie\":").count().max(1);
    let mut apps = Vec::new();
    let mut rest = trimmed;
    while let Some(pos) = rest.find("\"app\":\"") {
        let after = &rest[pos + 7..];
        if let Some(end) = after.find('"') {
            let app = after[..end].trim();
            if !app.is_empty() && !apps.iter().any(|a: &String| a == app) {
                apps.push(app.to_string());
            }
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    if apps.is_empty() {
        format!("Inhibited ({count} active)")
    } else {
        format!("Inhibited ({count}: {})", apps.join(", "))
    }
}
