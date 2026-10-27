// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Starting a scene: the engine session and the banner it animates.

use super::super::layout::{format_portrait_telemetry, is_portrait};
use super::super::sys_info::query_telemetry;
use super::super::{Ascii, EngineSession, SceneStep, draw, sys_info};
use crate::cell::TerminalCell;
use std::time::Duration;

impl Ascii {
    pub fn start_engine_session(&mut self) {
        crate::raster::blit::clear_subcell_overlay();
        let seed = self.rng.next_u64();
        self.backdrop_seed = draw::compute_backdrop_seed(self.effect as u64, seed);
        self.current_frame.clear();
        self.frame_grid.fill(TerminalCell::default());
        self.settled_bounds = None;
        if self.last_cols == 0 || self.last_rows == 0 {
            self.engine = None;
            return;
        }
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
        let slot = self.dwell_time();
        self.engine = EngineSession::new_paced(
            &self.art_text,
            self.effect,
            cols,
            rows,
            seed,
            self.fps,
            slot,
        );
        if let Some(ref mut session) = self.engine {
            if session.next_frame_into(&mut self.current_frame, Duration::ZERO)
                == SceneStep::Advanced
            {
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

    pub(super) fn render_static_banner(&mut self, text: &str, stacked: bool) {
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
}
