// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! State model and business logic for the settings TUI with OS-only default and UTF-8 safe editing.

pub use super::mode_choice::{ModeChoice, TOTAL_FIELDS, TuiField, char_to_byte_idx};
use crate::ascii::sys_info::{detect_desktop_env, detect_kernel_version, detect_os_name};
use crate::daemon::config::Config;

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

impl TuiModel {
    pub fn from_config(config: &Config) -> Self {
        let mut custom_text = config.saver.ascii.text.clone();
        let dwell_secs = config.saver.ascii.dwell_secs.clamp(0.5, 60.0);
        let idle_timeout_secs = config.general.idle_timeout_secs.clamp(30, 7200);
        let detected_os = detect_os_name();
        let detected_de = detect_desktop_env();
        let detected_kernel = detect_kernel_version();

        let mut target_checked = [false; 5];
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

        for item in &raw_items {
            match item.trim().to_ascii_lowercase().as_str() {
                "os" | "host" | "superfedora" | "sf" | "cycle" => target_checked[0] = true,
                "de" | "desktop" => target_checked[1] = true,
                "kernel" | "linux" => target_checked[2] = true,
                "idlescreen" | "wordmark" => target_checked[3] = true,
                "custom" | "text" => target_checked[4] = true,
                "" => {}
                other => {
                    if other.eq_ignore_ascii_case(&detected_os) {
                        target_checked[0] = true;
                    } else if other.eq_ignore_ascii_case(&detected_de) {
                        target_checked[1] = true;
                    } else if other.eq_ignore_ascii_case(&detected_kernel) {
                        target_checked[2] = true;
                    } else {
                        target_checked[4] = true;
                        if custom_text.is_empty() {
                            custom_text = other.to_string();
                        }
                    }
                }
            }
        }
        if !target_checked.iter().any(|&c| c) {
            target_checked[0] = true;
        }
        let first_idx = target_checked.iter().position(|&c| c).unwrap_or(0);
        let cursor_pos = custom_text.chars().count();
        let mut model = Self {
            mode: ModeChoice::ALL[first_idx],
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
        self.focused_index = (self.focused_index + TOTAL_FIELDS - 1) % TOTAL_FIELDS;
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
        let mut targets: Vec<String> = self
            .target_checked
            .iter()
            .enumerate()
            .filter(|&(_, &c)| c)
            .map(|(i, _)| ModeChoice::ALL[i].to_config_str().to_string())
            .collect();
        if targets.is_empty() {
            targets.push("os".to_string());
        }
        targets
    }

    pub fn selected_targets(&self) -> Vec<String> {
        self.active_targets()
    }

    pub fn resolved_words(&self) -> Vec<String> {
        let t = self.custom_text.trim();
        let custom = if t.is_empty() { "CUSTOM" } else { t };
        let (os, de) = (self.detected_os.as_str(), self.detected_de.as_str());
        let cands = [os, de, "LINUX", "IDLESCREEN", custom];
        let mut words: Vec<String> = self
            .target_checked
            .iter()
            .zip(cands)
            .filter(|&(&c, _)| c)
            .map(|(_, w)| w.to_string())
            .collect();
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
            TuiField::ModeChoice(idx) if self.target_checked[idx] => self.toggle_target(idx),
            TuiField::IdleTimeout => {
                self.idle_timeout_secs = self.idle_timeout_secs.saturating_sub(30).max(30);
                self.dirty = true;
            }
            TuiField::CustomText if self.cursor_pos > 0 => self.cursor_pos -= 1,
            _ => {}
        }
    }

    pub fn adjust_right(&mut self) {
        match self.current_field() {
            TuiField::ModeChoice(idx) if !self.target_checked[idx] => self.toggle_target(idx),
            TuiField::IdleTimeout => {
                self.idle_timeout_secs = (self.idle_timeout_secs + 30).min(7200);
                self.dirty = true;
            }
            TuiField::CustomText if self.cursor_pos < self.custom_text.chars().count() => {
                self.cursor_pos += 1;
            }
            _ => {}
        }
    }

    pub fn select_mode(&mut self, mode: ModeChoice) {
        let idx = ModeChoice::ALL.iter().position(|&m| m == mode).unwrap_or(0);
        self.focused_index = idx;
        self.toggle_target(idx);
    }

    fn mark_custom_edited(&mut self) {
        self.target_checked[4] = true;
        self.mode = ModeChoice::CustomText;
        self.dirty = true;
        self.update_status_sequence();
    }

    pub fn insert_char(&mut self, c: char) {
        let len = self.custom_text.chars().count();
        if len < 80 && !c.is_control() {
            let pos = self.cursor_pos.min(len);
            let b = char_to_byte_idx(&self.custom_text, pos);
            self.custom_text.insert(b, c);
            self.cursor_pos = pos + 1;
            self.mark_custom_edited();
        }
    }

    pub fn delete_back(&mut self) {
        let len = self.custom_text.chars().count();
        if self.cursor_pos > 0 && len > 0 {
            let pos = self.cursor_pos.min(len) - 1;
            let b = char_to_byte_idx(&self.custom_text, pos);
            self.custom_text.remove(b);
            self.cursor_pos = pos;
            self.mark_custom_edited();
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_pos < self.custom_text.chars().count() {
            let b = char_to_byte_idx(&self.custom_text, self.cursor_pos);
            self.custom_text.remove(b);
            self.mark_custom_edited();
        }
    }
}
