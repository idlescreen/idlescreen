// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! State model and business logic for the settings TUI with multi-select target support.

use crate::ascii::sys_info::{detect_desktop_env, detect_kernel_version, detect_os_name};
use crate::daemon::config::Config;

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
            "os" | "host" => ModeChoice::HostOs,
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

pub struct TuiModel {
    pub mode: ModeChoice,
    pub target_checked: [bool; 5],
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
        let mut custom_text = config.saver.ascii.text.clone();
        let dwell_secs = config.saver.ascii.dwell_secs.clamp(0.5, 60.0);
        let idle_timeout_secs = config.general.idle_timeout_secs.clamp(30, 7200);

        let detected_os = detect_os_name();
        let detected_de = detect_desktop_env();
        let detected_kernel = detect_kernel_version();
        let cursor_pos = custom_text.len();

        let mut target_checked = [false; 5];
        if config.saver.ascii.targets.is_empty() {
            let m = config.saver.ascii.mode.trim().to_ascii_lowercase();
            if m.contains(',') {
                for part in m.split(',') {
                    match part.trim() {
                        "os" | "host" => target_checked[0] = true,
                        "de" | "desktop" => target_checked[1] = true,
                        "kernel" | "linux" => target_checked[2] = true,
                        "idlescreen" | "wordmark" => target_checked[3] = true,
                        "custom" | "text" => target_checked[4] = true,
                        _ => {}
                    }
                }
            } else {
                match m.as_str() {
                    "os" | "host" => target_checked[0] = true,
                    "de" | "desktop" => target_checked[1] = true,
                    "kernel" | "linux" => target_checked[2] = true,
                    "idlescreen" | "wordmark" => target_checked[3] = true,
                    "custom" | "text" => target_checked[4] = true,
                    _ => {
                        // Default multi-target: Host OS and DE
                        target_checked[0] = true;
                        target_checked[1] = true;
                    }
                }
            }
        } else {
            for t in &config.saver.ascii.targets {
                match t.to_ascii_lowercase().as_str() {
                    "os" | "host" => target_checked[0] = true,
                    "de" | "desktop" => target_checked[1] = true,
                    "kernel" | "linux" => target_checked[2] = true,
                    "idlescreen" | "wordmark" => target_checked[3] = true,
                    "custom" | "text" => target_checked[4] = true,
                    other => {
                        if other.eq_ignore_ascii_case(&detected_os) {
                            target_checked[0] = true;
                        } else if other.eq_ignore_ascii_case(&detected_de) {
                            target_checked[1] = true;
                        } else if other.eq_ignore_ascii_case(&detected_kernel)
                            || other.eq_ignore_ascii_case("linux")
                        {
                            target_checked[2] = true;
                        } else if other.eq_ignore_ascii_case("idlescreen") {
                            target_checked[3] = true;
                        } else {
                            target_checked[4] = true;
                            if custom_text.is_empty() {
                                custom_text = other.to_string();
                            }
                        }
                    }
                }
            }
        }

        if !target_checked.iter().any(|&c| c) {
            target_checked[0] = true;
            target_checked[1] = true;
        }

        let mode = if target_checked[0] {
            ModeChoice::HostOs
        } else if target_checked[1] {
            ModeChoice::DesktopEnv
        } else if target_checked[2] {
            ModeChoice::Kernel
        } else if target_checked[3] {
            ModeChoice::Wordmark
        } else {
            ModeChoice::CustomText
        };

        let mut model = Self {
            mode,
            target_checked,
            custom_text,
            dwell_secs,
            idle_timeout_secs,
            focused_index: 0,
            editing_text: false,
            cursor_pos,
            status: "Loaded configuration".to_string(),
            detected_os,
            detected_de,
            detected_kernel,
            dirty: false,
            should_quit: false,
        };
        model.update_status_sequence();
        model
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

    pub fn toggle_target(&mut self, idx: usize) {
        if idx < 5 {
            let count = self.target_checked.iter().filter(|&&c| c).count();
            if self.target_checked[idx] && count <= 1 {
                self.status = "At least one ASCII text target must remain selected".to_string();
                return;
            }
            self.target_checked[idx] = !self.target_checked[idx];
            self.dirty = true;
            if self.target_checked[idx] {
                self.mode = ModeChoice::ALL[idx];
            }
            self.update_status_sequence();
        }
    }

    pub fn active_targets(&self) -> Vec<String> {
        let mut targets = Vec::new();
        for (i, &checked) in self.target_checked.iter().enumerate() {
            if checked {
                targets.push(ModeChoice::ALL[i].to_config_str().to_string());
            }
        }
        if targets.is_empty() {
            targets.push("os".to_string());
        }
        targets
    }

    pub fn selected_targets(&self) -> Vec<String> {
        self.active_targets()
    }

    pub fn resolved_words(&self) -> Vec<String> {
        let mut words = Vec::new();
        if self.target_checked[0] {
            words.push(self.detected_os.clone());
        }
        if self.target_checked[1] {
            words.push(self.detected_de.clone());
        }
        if self.target_checked[2] {
            words.push("LINUX".to_string());
        }
        if self.target_checked[3] {
            words.push("IDLESCREEN".to_string());
        }
        if self.target_checked[4] {
            let custom = if self.custom_text.trim().is_empty() {
                "CUSTOM".to_string()
            } else {
                self.custom_text.trim().to_string()
            };
            words.push(custom);
        }
        if words.is_empty() {
            words.push(self.detected_os.clone());
        }
        words
    }

    pub fn sequence_summary(&self) -> String {
        let words = self.resolved_words();
        if words.len() > 1 {
            format!("{} -> {}", words.join(" -> "), words[0])
        } else {
            format!("Single ({})", words[0])
        }
    }

    pub fn update_status_sequence(&mut self) {
        self.status = format!("Active sequence: {}", self.sequence_summary());
    }

    pub fn adjust_left(&mut self) {
        match self.current_field() {
            TuiField::ModeChoice(idx) => {
                if self.target_checked[idx] {
                    self.toggle_target(idx);
                }
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
            TuiField::ModeChoice(idx) => {
                if !self.target_checked[idx] {
                    self.toggle_target(idx);
                }
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
        let idx = match mode {
            ModeChoice::HostOs => 0,
            ModeChoice::DesktopEnv => 1,
            ModeChoice::Kernel => 2,
            ModeChoice::Wordmark => 3,
            ModeChoice::CustomText => 4,
        };
        self.focused_index = idx;
        self.toggle_target(idx);
    }

    pub fn insert_char(&mut self, c: char) {
        if self.custom_text.len() < 80 && !c.is_control() {
            self.custom_text.insert(self.cursor_pos, c);
            self.cursor_pos += 1;
            self.target_checked[4] = true;
            self.mode = ModeChoice::CustomText;
            self.dirty = true;
            self.update_status_sequence();
        }
    }

    pub fn delete_back(&mut self) {
        if self.cursor_pos > 0 && !self.custom_text.is_empty() {
            self.cursor_pos -= 1;
            self.custom_text.remove(self.cursor_pos);
            self.target_checked[4] = true;
            self.mode = ModeChoice::CustomText;
            self.dirty = true;
            self.update_status_sequence();
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_pos < self.custom_text.len() {
            self.custom_text.remove(self.cursor_pos);
            self.target_checked[4] = true;
            self.mode = ModeChoice::CustomText;
            self.dirty = true;
            self.update_status_sequence();
        }
    }
}
