// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Target mode choices and focusable field definitions for the settings TUI.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeChoice {
    HostOs,
    DesktopEnv,
    Kernel,
    Wordmark,
    CustomText,
}

impl ModeChoice {
    pub const ALL: [ModeChoice; 5] = [
        ModeChoice::HostOs,
        ModeChoice::DesktopEnv,
        ModeChoice::Kernel,
        ModeChoice::Wordmark,
        ModeChoice::CustomText,
    ];

    pub fn to_config_str(self) -> &'static str {
        match self {
            ModeChoice::HostOs => "os",
            ModeChoice::DesktopEnv => "de",
            ModeChoice::Kernel => "kernel",
            ModeChoice::Wordmark => "idlescreen",
            ModeChoice::CustomText => "custom",
        }
    }

    pub fn from_config_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "os" | "host" | "superfedora" | "sf" => ModeChoice::HostOs,
            "de" | "desktop" => ModeChoice::DesktopEnv,
            "kernel" | "linux" => ModeChoice::Kernel,
            "idlescreen" | "wordmark" => ModeChoice::Wordmark,
            "custom" | "text" => ModeChoice::CustomText,
            _ => ModeChoice::HostOs,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ModeChoice::HostOs => "Host OS",
            ModeChoice::DesktopEnv => "Desktop Environment (DE)",
            ModeChoice::Kernel => "Linux Kernel",
            ModeChoice::Wordmark => "Wordmark",
            ModeChoice::CustomText => "Custom Text",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuiField {
    ModeChoice(usize), // 0..=4 for checklist target items
    CustomText,        // 5
    IdleTimeout,       // 6
    SaveButton,        // 7
    PreviewButton,     // 8
    QuitButton,        // 9
}

pub const TOTAL_FIELDS: usize = 10;

pub fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map_or(s.len(), |(idx, _)| idx)
}
