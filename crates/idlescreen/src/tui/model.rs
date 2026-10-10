// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! State model and business logic for the settings TUI.

use crate::ascii::sys_info::{detect_desktop_env, detect_kernel_version, detect_os_name};
use crate::daemon::config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeChoice {
    HostOs,
    DesktopEnv,
    Kernel,
    All3Cycle,
    CustomText,
}

impl ModeChoice {
    pub const ALL: [ModeChoice; 5] = [
        ModeChoice::HostOs,
        ModeChoice::DesktopEnv,
        ModeChoice::Kernel,
        ModeChoice::All3Cycle,
        ModeChoice::CustomText,
    ];

    pub fn to_config_str(self) -> &'static str {
        match self {
            ModeChoice::HostOs => "os",
            ModeChoice::DesktopEnv => "de",
            ModeChoice::Kernel => "kernel",
            ModeChoice::All3Cycle => "cycle",
            ModeChoice::CustomText => "custom",
        }
    }

    pub fn from_config_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "os" | "host" => ModeChoice::HostOs,
            "de" | "desktop" => ModeChoice::DesktopEnv,
            "kernel" | "linux" => ModeChoice::Kernel,
            "custom" | "text" => ModeChoice::CustomText,
            _ => ModeChoice::All3Cycle,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ModeChoice::HostOs => "Host OS",
            ModeChoice::DesktopEnv => "Desktop Environment (DE)",
            ModeChoice::Kernel => "Linux Kernel",
            ModeChoice::All3Cycle => "All 3 (Cycle)",
            ModeChoice::CustomText => "Custom Text",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuiField {
    ModeChoice(usize), // 0..4 for radio buttons
    CustomText,
    IdleTimeout,
    SaveButton,
    PreviewButton,
    QuitButton,
}

pub struct TuiModel {
    pub mode: ModeChoice,
    pub custom_text: String,
    pub dwell_secs: f32,
    pub idle_timeout_secs: u64,
    pub focused_index: usize,
    pub editing_text: bool,
    pub cursor_pos: usize,
    pub status: String,
    pub detected_os: String,
    pub detected_de: String,
    pub detected_kernel: String,
    pub dirty: bool,
    pub should_quit: bool,
}

pub const TOTAL_FIELDS: usize = 10;

impl TuiModel {
    pub fn from_config(config: &Config) -> Self {
        let mode = ModeChoice::from_config_str(&config.saver.ascii.mode);
        let custom_text = config.saver.ascii.text.clone();
        let dwell_secs = config.saver.ascii.dwell_secs.clamp(0.5, 60.0);
        let idle_timeout_secs = config.general.idle_timeout_secs.clamp(30, 7200);

        let detected_os = detect_os_name();
        let detected_de = detect_desktop_env();
        let detected_kernel = detect_kernel_version();
        let cursor_pos = custom_text.len();

        let initial_mode_idx = match mode {
            ModeChoice::HostOs => 0,
            ModeChoice::DesktopEnv => 1,
            ModeChoice::Kernel => 2,
            ModeChoice::All3Cycle => 3,
            ModeChoice::CustomText => 4,
        };

        Self {
            mode,
            custom_text,
            dwell_secs,
            idle_timeout_secs,
            focused_index: initial_mode_idx,
            editing_text: false,
            cursor_pos,
            status: "Loaded configuration".to_string(),
            detected_os,
            detected_de,
            detected_kernel,
            dirty: false,
            should_quit: false,
        }
    }

    pub fn current_field(&self) -> TuiField {
        match self.focused_index {
            0..=4 => TuiField::ModeChoice(self.focused_index),
            5 => TuiField::CustomText,
            6 => TuiField::IdleTimeout,
            7 => TuiField::SaveButton,
            8 => TuiField::PreviewButton,
            _ => TuiField::QuitButton,
        }
    }

    pub fn nav_next(&mut self) {
        self.editing_text = false;
        self.focused_index = (self.focused_index + 1) % TOTAL_FIELDS;
    }

    pub fn nav_prev(&mut self) {
        self.editing_text = false;
        if self.focused_index == 0 {
            self.focused_index = TOTAL_FIELDS - 1;
        } else {
            self.focused_index -= 1;
        }
    }

    pub fn adjust_left(&mut self) {
        match self.current_field() {
            TuiField::ModeChoice(_) => {
                let current_idx = match self.mode {
                    ModeChoice::HostOs => 0,
                    ModeChoice::DesktopEnv => 1,
                    ModeChoice::Kernel => 2,
                    ModeChoice::All3Cycle => 3,
                    ModeChoice::CustomText => 4,
                };
                let prev_idx = if current_idx == 0 { 4 } else { current_idx - 1 };
                self.mode = ModeChoice::ALL[prev_idx];
                self.focused_index = prev_idx;
                self.dirty = true;
            }
            TuiField::IdleTimeout => {
                self.idle_timeout_secs = self.idle_timeout_secs.saturating_sub(30).max(30);
                self.dirty = true;
            }
            TuiField::CustomText if self.cursor_pos > 0 => {
                self.cursor_pos -= 1;
            }
            _ => {}
        }
    }

    pub fn adjust_right(&mut self) {
        match self.current_field() {
            TuiField::ModeChoice(_) => {
                let current_idx = match self.mode {
                    ModeChoice::HostOs => 0,
                    ModeChoice::DesktopEnv => 1,
                    ModeChoice::Kernel => 2,
                    ModeChoice::All3Cycle => 3,
                    ModeChoice::CustomText => 4,
                };
                let next_idx = (current_idx + 1) % 5;
                self.mode = ModeChoice::ALL[next_idx];
                self.focused_index = next_idx;
                self.dirty = true;
            }
            TuiField::IdleTimeout => {
                self.idle_timeout_secs = (self.idle_timeout_secs + 30).min(7200);
                self.dirty = true;
            }
            TuiField::CustomText if self.cursor_pos < self.custom_text.len() => {
                self.cursor_pos += 1;
            }
            _ => {}
        }
    }

    pub fn select_mode(&mut self, mode: ModeChoice) {
        self.mode = mode;
        self.focused_index = match mode {
            ModeChoice::HostOs => 0,
            ModeChoice::DesktopEnv => 1,
            ModeChoice::Kernel => 2,
            ModeChoice::All3Cycle => 3,
            ModeChoice::CustomText => 4,
        };
        self.dirty = true;
    }

    pub fn insert_char(&mut self, c: char) {
        if self.custom_text.len() < 80 && !c.is_control() {
            self.custom_text.insert(self.cursor_pos, c);
            self.cursor_pos += 1;
            self.mode = ModeChoice::CustomText;
            self.dirty = true;
        }
    }

    pub fn delete_back(&mut self) {
        if self.cursor_pos > 0 && !self.custom_text.is_empty() {
            self.cursor_pos -= 1;
            self.custom_text.remove(self.cursor_pos);
            self.mode = ModeChoice::CustomText;
            self.dirty = true;
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_pos < self.custom_text.len() {
            self.custom_text.remove(self.cursor_pos);
            self.mode = ModeChoice::CustomText;
            self.dirty = true;
        }
    }
}
