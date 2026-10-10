// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Built-in ASCII screensaver engine with dynamic tri-rotation mutation
//! and 37 visual text effects.

pub mod draw;
pub mod effects;
pub mod layout;
pub mod state;
pub mod sys_info;
pub mod unicode;

#[cfg(test)]
mod tests;

use crate::cell::{LcgRng, TerminalCell};
use draw::{paint_transition, place_banner_centered};
use effects::{EffectKind, EffectPlaylist};
use layout::{format_portrait_telemetry, is_portrait};
use state::{TextMode, TriRotationState};
use std::time::Duration;
use sys_info::query_telemetry;

pub const DEFAULT_DWELL_SECS: f32 = 5.0;
pub const DEFAULT_DISSOLVE_SECS: f32 = 0.7;

pub struct Ascii {
    pub rng: LcgRng,
    pub playlist: EffectPlaylist,
    pub rotation: TriRotationState,
    pub effect: EffectKind,
    pub pinned: bool,
    pub dwell_left: f32,
    pub dwell_duration: f32,
    pub elapsed_in_effect: f32,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
    pub last_cols: usize,
    pub last_rows: usize,
    pub target_grid: Vec<TerminalCell>,
    pub current_grid: Vec<TerminalCell>,
    pub transition_active: bool,
    pub transition_left: f32,
    pub transition_duration: f32,
    pub transition_from_grid: Vec<TerminalCell>,
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

    pub fn new_with_seed(seed: u64) -> Self {
        let mut rng = LcgRng::new(seed);
        let mut playlist = EffectPlaylist::new();
        let effect = playlist.next_effect(&mut rng);
        let rotation = TriRotationState::new_with_seed(TextMode::Cycle, None, seed);

        Self {
            rng,
            playlist,
            rotation,
            effect,
            pinned: false,
            dwell_left: DEFAULT_DWELL_SECS,
            dwell_duration: DEFAULT_DWELL_SECS,
            elapsed_in_effect: 0.0,
            fg: (248, 248, 242),
            bg: (0, 0, 0),
            last_cols: 0,
            last_rows: 0,
            target_grid: Vec::new(),
            current_grid: Vec::new(),
            transition_active: false,
            transition_left: 0.0,
            transition_duration: DEFAULT_DISSOLVE_SECS,
            transition_from_grid: Vec::new(),
        }
    }

    pub fn with_mode(mut self, mode: TextMode, custom_text: Option<String>) -> Self {
        self.rotation.mode = mode;
        self.rotation.custom_text = custom_text;
        self
    }

    pub fn with_dwell(mut self, dwell: f32) -> Self {
        let d = dwell.max(0.1);
        self.dwell_duration = d;
        self.dwell_left = d;
        self
    }

    pub fn pin_effect(&mut self, effect: EffectKind) {
        self.effect = effect;
        self.pinned = true;
        self.dwell_left = self.dwell_duration;
        self.elapsed_in_effect = 0.0;
    }

    pub fn init(&mut self, cols: usize, rows: usize) {
        self.last_cols = cols;
        self.last_rows = rows;
        let total = cols.saturating_mul(rows);
        self.target_grid.resize(total, TerminalCell::default());
        self.current_grid.resize(total, TerminalCell::default());
        self.transition_from_grid
            .resize(total, TerminalCell::default());
        self.rebuild_target();
    }

    pub fn rebuild_target(&mut self) {
        let cols = self.last_cols;
        let rows = self.last_rows;
        if cols == 0 || rows == 0 {
            return;
        }
        let total = cols * rows;
        self.target_grid.fill(TerminalCell::default());

        let active_text = self.rotation.active_text();
        let banner = self
            .rotation
            .atlas_cache
            .get_or_render(&active_text, None, cols, rows);

        let (_bx, _by, _bw, _bh) = place_banner_centered(
            &mut self.target_grid,
            cols,
            rows,
            &banner.lines,
            self.fg,
            self.bg,
        );

        // Portrait mode telemetry
        if is_portrait(cols, rows) {
            let telemetry = query_telemetry();
            let (top_lines, bottom_lines) = format_portrait_telemetry(&telemetry, cols);
            // Place top lines near row 1-2
            for (i, line) in top_lines.iter().enumerate() {
                let y = 1 + i;
                if y < rows {
                    for (x, ch) in line.chars().enumerate() {
                        if x < cols {
                            let idx = y * cols + x;
                            if idx < total && ch != ' ' {
                                self.target_grid[idx] = TerminalCell {
                                    ch,
                                    fg: (160, 160, 160),
                                    bg: self.bg,
                                    bold: false,
                                };
                            }
                        }
                    }
                }
            }
            // Place bottom lines near rows - 3
            let start_bottom = rows.saturating_sub(bottom_lines.len() + 1);
            for (i, line) in bottom_lines.iter().enumerate() {
                let y = start_bottom + i;
                if y < rows {
                    for (x, ch) in line.chars().enumerate() {
                        if x < cols {
                            let idx = y * cols + x;
                            if idx < total && ch != ' ' {
                                self.target_grid[idx] = TerminalCell {
                                    ch,
                                    fg: (160, 160, 160),
                                    bg: self.bg,
                                    bold: false,
                                };
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        if cols != self.last_cols || rows != self.last_rows {
            self.init(cols, rows);
        }

        let dt_secs = dt.as_secs_f32().clamp(0.0, 0.1);
        self.elapsed_in_effect += dt_secs;

        if self.transition_active {
            self.transition_left -= dt_secs;
            if self.transition_left <= 0.0 {
                self.transition_active = false;
                self.transition_left = 0.0;
            }
        }

        self.dwell_left -= dt_secs;
        if self.dwell_left <= 0.0 && !self.pinned {
            // Initiate transition to next effect and next brand target
            self.transition_active = true;
            self.transition_left = self.transition_duration;
            self.transition_from_grid
                .copy_from_slice(&self.current_grid);

            self.rotation.advance_mutation(&mut self.rng);
            self.effect = self.playlist.next_effect(&mut self.rng);
            self.dwell_left = self.dwell_duration;
            self.elapsed_in_effect = 0.0;
            self.rebuild_target();
        }

        // Render the current effect into current_grid
        self.effect.render(
            &self.target_grid,
            &mut self.current_grid,
            cols,
            rows,
            self.elapsed_in_effect,
        );
    }

    pub fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        let total = cols * rows;
        let n = total.min(grid.len()).min(self.current_grid.len());
        if n == 0 {
            return;
        }

        if self.transition_active && !self.transition_from_grid.is_empty() {
            let t = 1.0 - (self.transition_left / self.transition_duration).clamp(0.0, 1.0);
            paint_transition(
                &self.transition_from_grid,
                &self.current_grid,
                grid,
                t,
                cols,
                rows,
            );
        } else {
            grid[..n].copy_from_slice(&self.current_grid[..n]);
            if grid.len() > n {
                grid[n..].fill(TerminalCell::default());
            }
        }
    }
}
