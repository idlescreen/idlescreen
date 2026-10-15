// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! State model and business logic for the settings TUI with 50-scene selector and live status.

pub use super::mode_choice::{
    FPS_PRESETS, ModeChoice, TOTAL_FIELDS, TuiField, build_available_savers, char_to_byte_idx,
    format_dpms_display, format_fps_display, format_inhibitor_summary, format_saver_display,
    resolve_initial_mode_and_text,
};
use crate::ascii::sys_info::{detect_desktop_env, detect_kernel_version, detect_os_name};
use crate::daemon::config::Config;
use crate::daemon::control_sock::ControlClient;

#[allow(clippy::struct_excessive_bools)]
pub struct TuiModel {
    pub available_savers: Vec<String>,
    pub selected_saver_idx: usize,
    pub mode: ModeChoice,
    pub custom_text: String,
    pub dwell_secs: f32,
    pub idle_timeout_secs: u64,
    pub dpms_timeout_secs: u64,
    pub fps: u32,
    pub focused_index: usize,
    pub editing_text: bool,
    pub cursor_pos: usize,
    pub status: String,
    pub daemon_status: String,
    pub inhibitor_summary: String,
    pub detected_os: String,
    pub detected_de: String,
    pub detected_kernel: String,
    pub dirty: bool,
    pub quit_confirm_pending: bool,
    pub should_quit: bool,
}

impl TuiModel {
    pub fn from_config(config: &Config) -> Self {
        let dwell_secs = config.saver.ascii.dwell_secs.clamp(0.5, 30.0);
        let idle_timeout_secs = config.general.idle_timeout_secs.clamp(30, 7200);
        let dpms_timeout_secs = config.general.dpms_timeout_secs.min(7200);
        let fps = config.saver.ascii.fps;
        let (detected_os, detected_de, detected_kernel) = (
            detect_os_name(),
            detect_desktop_env(),
            detect_kernel_version(),
        );

        let available_savers = build_available_savers();
        let cfg_saver = crate::daemon::dbus::canonical_saver_name(
            config.saver.active.as_deref().unwrap_or("ascii"),
        );
        let selected_saver_idx = available_savers
            .iter()
            .position(|s| s == &cfg_saver)
            .or_else(|| available_savers.iter().position(|s| s == "ascii"))
            .unwrap_or(0);

        let (mode, custom_text) = resolve_initial_mode_and_text(config, &detected_os);
        let cursor_pos = custom_text.chars().count();
        let mut model = Self {
            available_savers,
            selected_saver_idx,
            mode,
            custom_text,
            dwell_secs,
            idle_timeout_secs,
            dpms_timeout_secs,
            fps,
            focused_index: 0,
            editing_text: false,
            cursor_pos,
            status: "Ready".to_string(),
            daemon_status: String::new(),
            inhibitor_summary: String::new(),
            detected_os,
            detected_de,
            detected_kernel,
            dirty: false,
            quit_confirm_pending: false,
            should_quit: false,
        };
        model.refresh_daemon_status();
        model.update_status_sequence();
        model
    }

    pub fn refresh_daemon_status(&mut self) {
        let sock_running = ControlClient::send_command("ping").is_ok_and(|r| r == "PONG");
        let svc_active = crate::cli::service::is_service_active();
        self.daemon_status = if sock_running || svc_active {
            "Active (Monitoring Idle)".to_string()
        } else {
            "Standby (Stay-Awake ON)".to_string()
        };
        self.inhibitor_summary = ControlClient::send_command("inhibitors").map_or_else(
            |_| "Uninhibited".to_string(),
            |r| format_inhibitor_summary(&r),
        );
    }

    pub fn active_saver_name(&self) -> &str {
        self.available_savers
            .get(self.selected_saver_idx)
            .map_or("ascii", String::as_str)
    }

    pub fn current_field(&self) -> TuiField {
        match self.focused_index {
            0 => TuiField::ActiveSaver,
            1 => TuiField::ModeChoice(0),
            2 => TuiField::ModeChoice(1),
            3 => TuiField::CustomText,
            4 => TuiField::IdleTimeout,
            5 => TuiField::DwellDuration,
            6 => TuiField::FrameRate,
            7 => TuiField::DpmsTimeout,
            8 => TuiField::SaveButton,
            9 => TuiField::PreviewButton,
            10 => TuiField::FullscreenButton,
            _ => TuiField::QuitButton,
        }
    }

    pub fn nav_next(&mut self) {
        self.editing_text = false;
        self.quit_confirm_pending = false;
        self.focused_index = (self.focused_index + 1) % TOTAL_FIELDS;
    }

    pub fn nav_prev(&mut self) {
        self.editing_text = false;
        self.quit_confirm_pending = false;
        self.focused_index = (self.focused_index + TOTAL_FIELDS - 1) % TOTAL_FIELDS;
    }

    pub fn set_mode(&mut self, choice: ModeChoice) {
        if self.mode != choice {
            self.mode = choice;
            self.dirty = true;
        }
        self.quit_confirm_pending = false;
        self.update_status_sequence();
    }

    pub fn select_mode(&mut self, choice: ModeChoice) {
        let idx = ModeChoice::ALL
            .iter()
            .position(|&m| m == choice)
            .unwrap_or(0);
        self.focused_index = 1 + idx;
        self.set_mode(choice);
    }

    pub fn active_targets(&self) -> Vec<String> {
        vec![self.mode.to_config_str().to_string()]
    }

    pub fn selected_targets(&self) -> Vec<String> {
        self.active_targets()
    }

    pub fn sequence_summary(&self) -> String {
        match self.mode {
            ModeChoice::HostOs => format!("OS Banner ({})", self.detected_os),
            ModeChoice::CustomText => {
                let t = self.custom_text.trim();
                format!(
                    "Custom Banner ({})",
                    if t.is_empty() { "CUSTOM" } else { t }
                )
            }
        }
    }

    pub fn update_status_sequence(&mut self) {
        self.status = format!(
            "Saver: {} | {}",
            self.active_saver_name(),
            self.sequence_summary()
        );
    }

    pub fn step_saver(&mut self, delta: isize) {
        let len = self.available_savers.len().max(1) as isize;
        self.selected_saver_idx =
            (self.selected_saver_idx as isize + delta).rem_euclid(len) as usize;
        self.dirty = true;
        self.quit_confirm_pending = false;
        self.update_status_sequence();
    }

    pub fn step_fps(&mut self, delta: isize) {
        let cur = FPS_PRESETS.iter().position(|&f| f == self.fps).unwrap_or(0) as isize;
        let len = FPS_PRESETS.len() as isize;
        self.fps = FPS_PRESETS[(cur + delta).rem_euclid(len) as usize];
        self.dirty = true;
        self.quit_confirm_pending = false;
    }

    pub fn adjust_by(&mut self, dir: isize, fast: bool) {
        self.quit_confirm_pending = false;
        match self.current_field() {
            TuiField::ActiveSaver => self.step_saver(if fast { dir * 5 } else { dir }),
            TuiField::ModeChoice(idx) => {
                if let Some(&choice) = ModeChoice::ALL.get(idx) {
                    self.set_mode(choice);
                }
            }
            TuiField::IdleTimeout => {
                let step = if fast { 300 } else { 30 };
                self.idle_timeout_secs = if dir < 0 {
                    self.idle_timeout_secs.saturating_sub(step).max(30)
                } else {
                    (self.idle_timeout_secs + step).min(7200)
                };
                self.dirty = true;
            }
            TuiField::DwellDuration => {
                let step = if fast { 2.0 } else { 0.5 };
                let next = self.dwell_secs + (dir as f32) * step;
                self.dwell_secs = ((next * 2.0).round() / 2.0).clamp(0.5, 30.0);
                self.dirty = true;
            }
            TuiField::FrameRate => self.step_fps(dir),
            TuiField::DpmsTimeout => {
                let step = if fast { 300 } else { 60 };
                self.dpms_timeout_secs = if dir < 0 {
                    let sub = self.dpms_timeout_secs.saturating_sub(step);
                    if sub < 60 { 0 } else { sub }
                } else {
                    (self.dpms_timeout_secs + step).min(7200)
                };
                self.dirty = true;
            }
            TuiField::CustomText => {
                if dir < 0 && self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                } else if dir > 0 && self.cursor_pos < self.custom_text.chars().count() {
                    self.cursor_pos += 1;
                }
            }
            _ => {}
        }
    }

    pub fn adjust_left(&mut self) {
        self.adjust_by(-1, false);
    }

    pub fn adjust_right(&mut self) {
        self.adjust_by(1, false);
    }
}
