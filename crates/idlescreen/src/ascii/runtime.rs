// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Frame update, engine session lifecycle, transition dissolve, and draw pass for Ascii.

use super::layout::{format_portrait_telemetry, is_portrait};
use super::sys_info::query_telemetry;
use super::{Ascii, EngineSession, draw, sys_info};
use crate::cell::TerminalCell;
use std::time::Duration;

impl Ascii {
    pub fn start_engine_session(&mut self) {
        crate::wayland::blit::clear_subcell_overlay();
        let seed = self.rng.next_u64();
        self.backdrop_seed = draw::compute_backdrop_seed(self.effect as u64, seed);
        self.current_frame.clear();
        self.frame_grid.fill(TerminalCell::default());
        self.settled_bounds = None;
        if self.last_cols == 0 || self.last_rows == 0 {
            self.engine = None;
            return;
        }
        self.anim_left = 120.0;
        self.rebuild_art();
        let is_sf = self.is_stacked_superfedora() || sys_info::is_superfedora_machine();
        if is_portrait(self.last_cols, self.last_rows) && !is_sf {
            let (top, bottom) = format_portrait_telemetry(&query_telemetry(), self.last_cols);
            (self.portrait_top, self.portrait_bottom) = (top, bottom);
        } else {
            self.portrait_top.clear();
            self.portrait_bottom.clear();
        }
        if self.art_text.trim().is_empty() {
            self.engine = None;
            return;
        }
        let (cols, rows) = (self.last_cols, self.last_rows);
        self.engine = EngineSession::new(&self.art_text, self.effect, cols, rows, seed, self.fps);
        if let Some(ref mut session) = self.engine {
            if let Some(frame) = session.next_frame() {
                self.current_frame.clear();
                self.current_frame.push_str(&frame);
                let (fg, bg) = (self.fg, self.bg);
                draw::paint_frame(
                    &self.current_frame,
                    &mut self.frame_grid,
                    cols,
                    rows,
                    fg,
                    bg,
                );
            }
        } else {
            let active_text = self.rotation.active_text();
            self.render_static_banner(&active_text, self.is_stacked_superfedora());
        }
    }

    fn render_static_banner(&mut self, text: &str, stacked: bool) {
        let (cols, rows, fg, bg) = (self.last_cols, self.last_rows, self.fg, self.bg);
        let banner = self
            .rotation
            .atlas_cache
            .get_or_render(text, None, cols, rows);
        self.frame_grid.fill(TerminalCell::default());
        let _ =
            draw::place_banner_centered(&mut self.frame_grid, cols, rows, &banner.lines, fg, bg);
        self.settled_bounds =
            draw::apply_settled_bevel_for(&mut self.frame_grid, cols, rows, stacked);
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
                (self.transition_active, self.transition_left) = (false, 0.0);
                self.transition_from_grid.clear();
                self.transition_from_grid.shrink_to_fit();
                if self.rotation.targets.len() > 1 {
                    self.cycle_effect();
                }
            }
        }

        let is_boss_scene = matches!(
            self.effect,
            super::effects::EffectKind::Mmx | super::effects::EffectKind::Zero
        );
        if let Some(ref mut session) = self.engine {
            if self.transition_active && is_boss_scene {
                return;
            }
            self.anim_left -= dt_secs;
            if let Some(frame) = (self.anim_left > 0.0)
                .then(|| session.next_frame())
                .flatten()
            {
                self.current_frame.clear();
                self.current_frame.push_str(&frame);
                let (fg, bg) = (self.fg, self.bg);
                draw::paint_frame(
                    &self.current_frame,
                    &mut self.frame_grid,
                    cols,
                    rows,
                    fg,
                    bg,
                );
            } else {
                let stacked = self.is_stacked_superfedora();
                self.settled_bounds =
                    draw::apply_settled_bevel_for(&mut self.frame_grid, cols, rows, stacked);
                (self.engine, self.dwell_left) = (None, self.dwell_time());
            }
        } else if !self.transition_active {
            self.dwell_left -= dt_secs;
            if self.dwell_left <= 0.0 && (!self.pinned || is_boss_scene) {
                self.begin_transition();
            }
        }
    }

    pub fn begin_transition(&mut self) {
        if self.frame_grid.is_empty() {
            self.cycle_effect();
            return;
        }
        let (cols, rows, sf) = (
            self.last_cols,
            self.last_rows,
            self.is_stacked_superfedora(),
        );
        let _ = draw::apply_settled_bevel_for(&mut self.frame_grid, cols, rows, sf);
        self.transition_from_grid.clone_from(&self.frame_grid);
        let dur = self.dissolve_duration.min(self.dwell_time() * 0.4);
        (
            self.transition_duration,
            self.transition_left,
            self.transition_active,
        ) = (dur, dur, true);

        if self.rotation.targets.len() > 1 {
            self.engine = None;
            let next_text = self.rotation.peek_next_text();
            let t = next_text.trim();
            let next_sf =
                t.eq_ignore_ascii_case("SUPERFEDORA") || t.eq_ignore_ascii_case("SUPER FEDORA");
            self.render_static_banner(&next_text, next_sf);
        } else {
            self.cycle_effect();
        }
    }

    pub fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        let n = (cols * rows).min(grid.len()).min(self.frame_grid.len());
        if n == 0 {
            grid.fill(TerminalCell::default());
            return;
        }

        if self.transition_active
            && self.transition_from_grid.len() == grid.len()
            && self.frame_grid.len() == grid.len()
        {
            let t = if self.transition_duration > 0.0 {
                (1.0 - (self.transition_left / self.transition_duration)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let from = &self.transition_from_grid;
            draw::paint_transition(from, &self.frame_grid, grid, t, cols, rows);
        } else if !self.frame_grid.is_empty() {
            let src_dim = (self.last_cols, self.last_rows);
            copy_frame_grid(&self.frame_grid, src_dim, grid, (cols, rows));
        } else {
            grid.fill(TerminalCell::default());
        }

        let bounds = self.is_settled().then_some(self.settled_bounds).flatten();
        let (tick, seed, sf, bg) = (
            self.frame_tick,
            self.backdrop_seed,
            self.is_stacked_superfedora(),
            self.bg,
        );
        draw::apply_arcade_fx_with_bounds(grid, cols, rows, tick, seed, bounds, sf, bg);

        if is_portrait(cols, rows) && !self.portrait_top.is_empty() && !sf {
            stamp_portrait_lines(grid, (cols, rows), 1, &self.portrait_top, bg);
            let start_b = rows.saturating_sub(self.portrait_bottom.len() + 1);
            stamp_portrait_lines(grid, (cols, rows), start_b, &self.portrait_bottom, bg);
        }
    }
}

fn copy_frame_grid(
    src: &[TerminalCell],
    (src_cols, src_rows): (usize, usize),
    dst: &mut [TerminalCell],
    (cols, rows): (usize, usize),
) {
    if src_cols == cols && src_rows == rows {
        let n = src.len().min(dst.len());
        dst[..n].copy_from_slice(&src[..n]);
        dst[n..].fill(TerminalCell::default());
    } else if src_cols > 0 && cols > 0 {
        dst.fill(TerminalCell::default());
        let (copy_c, copy_r) = (cols.min(src_cols), rows.min(src_rows));
        let (sx, dx) = (
            src_cols.saturating_sub(cols) / 2,
            cols.saturating_sub(src_cols) / 2,
        );
        let (sy, dy) = (
            src_rows.saturating_sub(rows) / 2,
            rows.saturating_sub(src_rows) / 2,
        );
        for y in 0..copy_r {
            let (s_idx, d_idx) = ((y + sy) * src_cols + sx, (y + dy) * cols + dx);
            if s_idx + copy_c <= src.len() && d_idx + copy_c <= dst.len() {
                dst[d_idx..d_idx + copy_c].copy_from_slice(&src[s_idx..s_idx + copy_c]);
            }
        }
    }
}

fn stamp_portrait_lines(
    grid: &mut [TerminalCell],
    (cols, rows): (usize, usize),
    start_y: usize,
    lines: &[String],
    bg: (u8, u8, u8),
) {
    for (i, line) in lines.iter().enumerate() {
        let y = start_y + i;
        if y < rows {
            for (x, ch) in line.chars().enumerate() {
                if x < cols
                    && ch != ' '
                    && let Some(cell) = grid.get_mut(y * cols + x)
                {
                    cell.ch = draw::pixel_art_glyph(ch, x, y);
                    (cell.fg, cell.bg, cell.bold) = ((81, 162, 218), bg, false);
                }
            }
        }
    }
}
