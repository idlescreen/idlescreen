// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Built-in ASCII screensaver engine with dynamic tri-rotation mutation
//! and 53 genuine 16-bit arcade visual text effects.

pub mod draw;
pub mod effects;
pub mod engine;
pub mod layout;
mod runtime;
pub mod scenes;
pub mod state;
pub mod sys_info;
pub mod unicode;

#[cfg(test)]
mod tests;

use crate::cell::{LcgRng, TerminalCell};
use effects::{EffectKind, EffectPlaylist};
pub use engine::EngineSession;
use state::{TextMode, TriRotationState};
use std::time::Duration;

pub const DEFAULT_DWELL_SECS: f32 = 2.8;
pub const DEFAULT_DISSOLVE_SECS: f32 = 0.50;

#[allow(clippy::struct_excessive_bools)]
pub struct Ascii {
    pub rng: LcgRng,
    pub playlist: EffectPlaylist,
    pub rotation: TriRotationState,
    pub effect: EffectKind,
    pub pinned: bool,
    pub tour_mode: bool,
    pub dwell_left: f32,
    pub dwell_duration: Option<f32>,
    pub dissolve_duration: f32,
    pub elapsed_in_effect: f32,
    pub anim_left: f32,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
    pub last_cols: usize,
    pub last_rows: usize,
    pub art_text: String,
    pub current_frame: String,
    pub frame_grid: Vec<TerminalCell>,
    pub settled_bounds: Option<(usize, usize)>,
    pub transition_active: bool,
    pub transition_left: f32,
    pub transition_duration: f32,
    pub transition_from_grid: Vec<TerminalCell>,
    pub engine: Option<EngineSession>,
    pub portrait_top: Vec<String>,
    pub portrait_bottom: Vec<String>,
    pub fps: u32,
    pub frame_tick: u64,
    pub backdrop_seed: u64,
}

impl Default for Ascii {
    fn default() -> Self {
        Self::new()
    }
}

impl Ascii {
    pub fn new() -> Self {
        Self::new_with_seed(LcgRng::new_random().next_u64())
    }

    pub fn new_superfedora() -> Self {
        let seed = LcgRng::new_random().next_u64();
        let mut saver = Self::new_with_seed(seed);
        saver.fg = (81, 162, 218);
        saver.bg = sys_info::detect_system_theme().bg_rgb;
        saver.rotation.os_name = "SUPERFEDORA".to_string();
        saver.rotation.mode = TextMode::Os;
        saver.rotation.custom_text = None;
        saver.rotation.set_targets(vec!["superfedora".to_string()]);
        saver.pinned = false;
        (saver.dwell_duration, saver.dwell_left) = (Some(2.6), 2.6);
        (saver.dissolve_duration, saver.transition_duration) =
            (DEFAULT_DISSOLVE_SECS, DEFAULT_DISSOLVE_SECS);
        saver
    }

    pub fn new_with_seed(seed: u64) -> Self {
        let mut rng = LcgRng::new(seed);
        let mut playlist = EffectPlaylist::new();
        let effect = playlist.next_effect(&mut rng);
        let rotation = TriRotationState::new_with_seed(TextMode::Os, None, seed);
        let backdrop_seed = draw::compute_backdrop_seed(effect as u64, seed);
        let bg = sys_info::detect_system_theme().bg_rgb;

        Self {
            rng,
            playlist,
            rotation,
            effect,
            pinned: false,
            tour_mode: false,
            dwell_left: DEFAULT_DWELL_SECS,
            dwell_duration: None,
            dissolve_duration: DEFAULT_DISSOLVE_SECS,
            elapsed_in_effect: 0.0,
            anim_left: 20.0,
            fg: (248, 248, 242),
            bg,
            last_cols: 0,
            last_rows: 0,
            art_text: String::new(),
            current_frame: String::new(),
            frame_grid: Vec::new(),
            settled_bounds: None,
            transition_active: false,
            transition_left: 0.0,
            transition_duration: DEFAULT_DISSOLVE_SECS,
            transition_from_grid: Vec::new(),
            engine: None,
            portrait_top: Vec::new(),
            portrait_bottom: Vec::new(),
            fps: 60,
            frame_tick: 0,
            backdrop_seed,
        }
    }

    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = if fps > 0 { fps } else { 60 };
        self
    }

    pub fn with_mode(mut self, mode: TextMode, custom_text: Option<String>) -> Self {
        self.rotation.mode = mode;
        self.rotation.custom_text = custom_text;
        self.rotation.sync_targets_from_mode();
        self
    }

    pub fn with_targets(mut self, targets: Vec<String>) -> Self {
        self.rotation.set_targets(targets);
        self
    }

    pub fn with_dwell(mut self, dwell: f32) -> Self {
        let d = dwell.max(0.1);
        self.dwell_duration = Some(d);
        self.dwell_left = d;
        self
    }

    pub fn with_dissolve(mut self, dissolve_secs: f32) -> Self {
        let d = dissolve_secs.clamp(0.05, 5.0);
        self.dissolve_duration = d;
        self.transition_duration = d;
        self
    }

    pub fn dwell_time(&self) -> f32 {
        let d = self.dwell_duration.unwrap_or_else(|| self.effect.dwell());
        if matches!(self.effect, EffectKind::Mmx | EffectKind::Zero) {
            d.min(0.65)
        } else {
            d
        }
    }

    pub fn is_settled(&self) -> bool {
        self.engine.is_none() && !self.transition_active
    }

    pub fn is_stacked_superfedora(&self) -> bool {
        let active = self.rotation.active_text();
        let trimmed = active.trim();
        trimmed.eq_ignore_ascii_case("SUPERFEDORA") || trimmed.eq_ignore_ascii_case("SUPER FEDORA")
    }

    pub fn dwell_remaining(&self) -> Option<Duration> {
        if self.is_settled() && self.dwell_left > 0.0 {
            Some(Duration::from_secs_f32(self.dwell_left))
        } else {
            None
        }
    }

    pub fn pin_effect(&mut self, effect: EffectKind) {
        (self.effect, self.pinned, self.tour_mode) = (effect, true, false);
        (self.dwell_left, self.elapsed_in_effect) = (self.dwell_time(), 0.0);
        self.start_engine_session();
    }

    pub fn enable_tour_mode(&mut self) {
        (self.effect, self.pinned, self.tour_mode) = (EffectKind::ALL[0], false, true);
        (self.dwell_left, self.elapsed_in_effect) = (self.dwell_time(), 0.0);
        self.start_engine_session();
    }

    pub fn step_effect(&mut self, roster: &[EffectKind], delta: i32) {
        self.effect = self.effect.step_in_roster(roster, delta);
        (self.transition_active, self.transition_left) = (false, 0.0);
        self.transition_from_grid.clear();
        (self.dwell_left, self.elapsed_in_effect) = (self.dwell_time(), 0.0);
        self.start_engine_session();
    }

    pub fn init(&mut self, cols: usize, rows: usize) {
        (self.last_cols, self.last_rows) = (cols, rows);
        let total = cols.saturating_mul(rows);
        self.frame_grid.resize(total, TerminalCell::default());
        self.frame_grid.fill(TerminalCell::default());
        self.settled_bounds = None;
        self.transition_from_grid.clear();
        (self.transition_active, self.transition_left) = (false, 0.0);
        self.dwell_left = self.dwell_time();
        self.start_engine_session();
    }

    pub fn rebuild_art(&mut self) {
        if self.last_cols == 0 || self.last_rows == 0 {
            self.art_text.clear();
            return;
        }
        let active_text = self.rotation.active_text();
        let banner = self.rotation.atlas_cache.get_or_render(
            &active_text,
            None,
            self.last_cols,
            self.last_rows,
        );
        if !banner.lines.is_empty() && banner.lines.iter().any(|l| !l.trim().is_empty()) {
            self.art_text = banner.lines.join("\n");
        } else if !active_text.trim().is_empty() {
            self.art_text = active_text;
        } else {
            self.art_text = "IDLESCREEN".to_string();
        }
    }

    pub fn cycle_effect(&mut self) {
        self.rotation.advance_mutation(&mut self.rng);
        if !self.pinned {
            self.effect = if self.tour_mode {
                self.effect.step_in_roster(&EffectKind::ALL, 1)
            } else {
                self.playlist.next_effect(&mut self.rng)
            };
        }
        (self.dwell_left, self.elapsed_in_effect) = (self.dwell_time(), 0.0);
        self.start_engine_session();
    }
}
