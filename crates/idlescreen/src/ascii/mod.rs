// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Built-in ASCII screensaver engine with dynamic tri-rotation mutation
//! and 37 genuine ttfx visual text effects.

pub mod draw;
pub mod effects;
pub mod engine;
pub mod layout;
#[allow(clippy::all, unused)]
pub mod scenes;
pub mod state;
pub mod sys_info;
pub mod unicode;

#[cfg(test)]
mod tests;

use crate::cell::{LcgRng, TerminalCell};
use effects::{EffectKind, EffectPlaylist};
pub use engine::EngineSession;
use layout::{format_portrait_telemetry, is_portrait};
use state::{TextMode, TriRotationState};
use std::time::Duration;
use sys_info::query_telemetry;

pub const DEFAULT_DWELL_SECS: f32 = 2.0;
pub const DEFAULT_DISSOLVE_SECS: f32 = 0.35;

pub struct Ascii {
    pub rng: LcgRng,
    pub playlist: EffectPlaylist,
    pub rotation: TriRotationState,
    pub effect: EffectKind,
    pub pinned: bool,
    pub dwell_left: f32,
    pub dwell_duration: Option<f32>,
    pub elapsed_in_effect: f32,
    pub anim_left: f32,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
    pub last_cols: usize,
    pub last_rows: usize,
    pub art_text: String,
    pub current_frame: String,
    pub frame_grid: Vec<TerminalCell>,
    pub transition_active: bool,
    pub transition_left: f32,
    pub transition_duration: f32,
    pub transition_from_grid: Vec<TerminalCell>,
    pub engine: Option<EngineSession>,
    pub portrait_top: Vec<String>,
    pub portrait_bottom: Vec<String>,
    pub fps: u32,
    pub frame_tick: u64,
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

    /// Dedicated 16-bit SuperFedora arcade screensaver preset with Tokyo Night jewel colors,
    /// stacked 2D pixel-art website banner, and randomized 37-scene arcade rotation.
    pub fn new_superfedora() -> Self {
        let seed = LcgRng::new_random().next_u64();
        let mut saver = Self::new_with_seed(seed);
        saver.fg = (81, 162, 218); // Fedora Cyan #51a2da
        saver.bg = (0, 0, 0);
        saver.rotation.os_name = "SUPERFEDORA".to_string();
        saver.rotation.mode = TextMode::Custom;
        saver.rotation.custom_text = Some("SUPERFEDORA".to_string());
        saver.rotation.set_targets(vec!["superfedora".to_string()]);
        saver.pinned = false;
        saver.dwell_duration = Some(1.8);
        saver.dwell_left = 1.8;
        saver.transition_duration = DEFAULT_DISSOLVE_SECS;
        saver
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
            dwell_duration: None,
            elapsed_in_effect: 0.0,
            anim_left: 20.0,
            fg: (248, 248, 242),
            bg: (0, 0, 0),
            last_cols: 0,
            last_rows: 0,
            art_text: String::new(),
            current_frame: String::new(),
            frame_grid: Vec::new(),
            transition_active: false,
            transition_left: 0.0,
            transition_duration: DEFAULT_DISSOLVE_SECS,
            transition_from_grid: Vec::new(),
            engine: None,
            portrait_top: Vec::new(),
            portrait_bottom: Vec::new(),
            fps: 60,
            frame_tick: 0,
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

    pub fn dwell_time(&self) -> f32 {
        self.dwell_duration.unwrap_or_else(|| self.effect.dwell())
    }

    /// Returns true when the scene is settled in its post-transition dwell period (zero motion).
    pub fn is_settled(&self) -> bool {
        self.engine.is_none() && !self.transition_active
    }

    /// Returns the remaining dwell duration if currently settled.
    pub fn dwell_remaining(&self) -> Option<Duration> {
        if self.is_settled() && self.dwell_left > 0.0 {
            Some(Duration::from_secs_f32(self.dwell_left))
        } else {
            None
        }
    }

    pub fn pin_effect(&mut self, effect: EffectKind) {
        self.effect = effect;
        self.pinned = true;
        self.dwell_left = self.dwell_time();
        self.elapsed_in_effect = 0.0;
        self.start_engine_session();
    }

    pub fn init(&mut self, cols: usize, rows: usize) {
        self.last_cols = cols;
        self.last_rows = rows;
        let total = cols.saturating_mul(rows);
        self.frame_grid.resize(total, TerminalCell::default());
        self.frame_grid.fill(TerminalCell::default());
        self.transition_from_grid.clear();
        self.transition_active = false;
        self.transition_left = 0.0;
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

    pub fn start_engine_session(&mut self) {
        self.current_frame.clear();
        self.frame_grid.fill(TerminalCell::default());
        if self.last_cols == 0 || self.last_rows == 0 {
            self.engine = None;
            return;
        }
        self.anim_left = 120.0;
        self.rebuild_art();
        let is_superfedora_banner = self
            .rotation
            .active_text()
            .eq_ignore_ascii_case("SUPERFEDORA")
            || sys_info::is_superfedora_machine();
        if is_portrait(self.last_cols, self.last_rows) && !is_superfedora_banner {
            let telemetry = query_telemetry();
            let (top, bottom) = format_portrait_telemetry(&telemetry, self.last_cols);
            self.portrait_top = top;
            self.portrait_bottom = bottom;
        } else {
            self.portrait_top.clear();
            self.portrait_bottom.clear();
        }
        if self.art_text.trim().is_empty() {
            self.engine = None;
            return;
        }
        let seed = self.rng.next_u64();
        self.engine = EngineSession::new(
            &self.art_text,
            self.effect,
            self.last_cols,
            self.last_rows,
            seed,
            self.fps,
        );
        if let Some(ref mut session) = self.engine {
            if let Some(frame) = session.next_frame() {
                self.current_frame.clear();
                self.current_frame.push_str(&frame);
                draw::paint_frame(
                    &self.current_frame,
                    &mut self.frame_grid,
                    self.last_cols,
                    self.last_rows,
                    self.fg,
                    self.bg,
                );
            }
        } else {
            // Fallback for dimensions where ttfx cannot initialize: place centered banner
            let active_text = self.rotation.active_text();
            let banner = self.rotation.atlas_cache.get_or_render(
                &active_text,
                None,
                self.last_cols,
                self.last_rows,
            );
            let _ = draw::place_banner_centered(
                &mut self.frame_grid,
                self.last_cols,
                self.last_rows,
                &banner.lines,
                self.fg,
                self.bg,
            );
        }
    }

    pub fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        if cols != self.last_cols || rows != self.last_rows {
            self.init(cols, rows);
        }

        self.frame_tick = self.frame_tick.wrapping_add(1);
        let dt_secs = dt.as_secs_f32().clamp(0.0, 0.1);
        self.elapsed_in_effect += dt_secs;

        if self.transition_active {
            self.transition_left -= dt_secs;
            if self.transition_left <= 0.0 {
                self.transition_active = false;
                self.transition_left = 0.0;
                self.transition_from_grid.clear();
                if self.rotation.targets.len() > 1 {
                    self.cycle_effect();
                }
            }
        }

        if let Some(ref mut session) = self.engine {
            self.anim_left -= dt_secs;
            let frame = if self.anim_left > 0.0 {
                session.next_frame()
            } else {
                None
            };
            if let Some(frame) = frame {
                self.current_frame.clear();
                self.current_frame.push_str(&frame);
                draw::paint_frame(
                    &self.current_frame,
                    &mut self.frame_grid,
                    cols,
                    rows,
                    self.fg,
                    self.bg,
                );
            } else {
                // Animation complete: drop session and dwell on settled frame
                self.engine = None;
                self.dwell_left = self.dwell_time();
            }
        } else if !self.transition_active {
            // Banner is settled: countdown dwell
            self.dwell_left -= dt_secs;
            if self.dwell_left <= 0.0 && !self.pinned {
                self.begin_transition();
            }
        }
    }

    pub fn begin_transition(&mut self) {
        if self.frame_grid.is_empty() {
            self.cycle_effect();
            return;
        }
        self.transition_from_grid.clone_from(&self.frame_grid);
        let dur = DEFAULT_DISSOLVE_SECS.min(self.dwell_time() * 0.4);
        self.transition_duration = dur;
        self.transition_left = dur;
        self.transition_active = true;

        if self.rotation.targets.len() > 1 {
            // Conclude scene by transitioning directly to the next target banner.
            self.engine = None;
            let next_text = self.rotation.peek_next_text();
            let banner = self.rotation.atlas_cache.get_or_render(
                &next_text,
                None,
                self.last_cols,
                self.last_rows,
            );
            self.frame_grid.fill(TerminalCell::default());
            let _ = draw::place_banner_centered(
                &mut self.frame_grid,
                self.last_cols,
                self.last_rows,
                &banner.lines,
                self.fg,
                self.bg,
            );
        } else {
            self.cycle_effect();
        }
    }

    pub fn cycle_effect(&mut self) {
        self.rotation.advance_mutation(&mut self.rng);
        if !self.pinned {
            self.effect = self.playlist.next_effect(&mut self.rng);
        }
        self.dwell_left = self.dwell_time();
        self.elapsed_in_effect = 0.0;
        self.start_engine_session();
    }

    pub fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        let total = cols * rows;
        let n = total.min(grid.len()).min(self.frame_grid.len());
        if n == 0 {
            grid.fill(TerminalCell::default());
            return;
        }

        if self.transition_active
            && !self.transition_from_grid.is_empty()
            && self.transition_from_grid.len() == grid.len()
            && self.frame_grid.len() == grid.len()
        {
            let t = if self.transition_duration > 0.0 {
                (1.0 - (self.transition_left / self.transition_duration)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            draw::paint_transition(
                &self.transition_from_grid,
                &self.frame_grid,
                grid,
                t,
                cols,
                rows,
            );
        } else if !self.frame_grid.is_empty() {
            if self.last_cols == cols && self.last_rows == rows {
                let n = self.frame_grid.len().min(grid.len());
                grid[..n].copy_from_slice(&self.frame_grid[..n]);
                if grid.len() > n {
                    grid[n..].fill(TerminalCell::default());
                }
            } else if self.last_cols > 0 && cols > 0 {
                grid.fill(TerminalCell::default());
                let copy_cols = cols.min(self.last_cols);
                let copy_rows = rows.min(self.last_rows);
                for y in 0..copy_rows {
                    let src_start = y * self.last_cols;
                    let dst_start = y * cols;
                    if src_start + copy_cols <= self.frame_grid.len()
                        && dst_start + copy_cols <= grid.len()
                    {
                        grid[dst_start..dst_start + copy_cols]
                            .copy_from_slice(&self.frame_grid[src_start..src_start + copy_cols]);
                    }
                }
            } else {
                let n = self.frame_grid.len().min(grid.len());
                grid[..n].copy_from_slice(&self.frame_grid[..n]);
                if grid.len() > n {
                    grid[n..].fill(TerminalCell::default());
                }
            }
        } else {
            grid.fill(TerminalCell::default());
        }

        draw::apply_arcade_fx(
            grid,
            cols,
            rows,
            self.frame_tick,
            self.is_settled(),
            self.bg,
        );

        if is_portrait(cols, rows)
            && !self.portrait_top.is_empty()
            && !self
                .rotation
                .active_text()
                .eq_ignore_ascii_case("SUPERFEDORA")
        {
            for (i, line) in self.portrait_top.iter().enumerate() {
                let y = 1 + i;
                if y < rows {
                    for (x, ch) in line.chars().enumerate() {
                        if x < cols {
                            let idx = y * cols + x;
                            if idx < grid.len() && ch != ' ' {
                                grid[idx] = TerminalCell {
                                    ch: draw::pixel_art_glyph(ch, x, y),
                                    fg: (81, 162, 218),
                                    bg: self.bg,
                                    bold: false,
                                };
                            }
                        }
                    }
                }
            }
            let start_bottom = rows.saturating_sub(self.portrait_bottom.len() + 1);
            for (i, line) in self.portrait_bottom.iter().enumerate() {
                let y = start_bottom + i;
                if y < rows {
                    for (x, ch) in line.chars().enumerate() {
                        if x < cols {
                            let idx = y * cols + x;
                            if idx < grid.len() && ch != ' ' {
                                grid[idx] = TerminalCell {
                                    ch: draw::pixel_art_glyph(ch, x, y),
                                    fg: (81, 162, 218),
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
}
