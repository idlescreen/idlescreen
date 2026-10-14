// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl VhsTape {
    /// Line.snow.
    pub(super) fn line_snow(&mut self, ctx: &mut EngineCtx, idx: usize) {
        let characters = self.lines[idx].characters.clone();
        for id in characters {
            ctx.activate_scene(self, id, "snow");
        }
    }

    /// Line.set_hold_time.
    pub(super) fn line_set_hold_time(&mut self, ctx: &mut EngineCtx, idx: usize, hold_time: i64) {
        for &id in &self.lines[idx].characters {
            ctx.terminal.arena[id.0 as usize]
                .motion
                .paths
                .get_mut("glitch")
                .must_val()
                .hold_time = hold_time;
        }
    }

    /// Line.glitch.
    pub(super) fn line_glitch(&mut self, ctx: &mut EngineCtx, idx: usize, final_: bool) {
        let characters = self.lines[idx].characters.clone();
        for id in characters {
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                if final_ {
                    ch.motion.paths.get_mut("glitch").must_val().hold_time = 0;
                    ch.motion.paths.get_mut("restore").must_val().hold_time = 0;
                }
            }
            let glitch_speed = 40.0 / ctx.rng.randint(20, 40) as f64;
            let restore_speed = 40.0 / ctx.rng.randint(20, 40) as f64;
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion.paths.get_mut("glitch").must_val().speed = glitch_speed;
                ch.motion.paths.get_mut("restore").must_val().speed = restore_speed;
            }
            ctx.activate_path(self, id, "glitch");
        }
    }

    /// Line.restore.
    pub(super) fn line_restore(&mut self, ctx: &mut EngineCtx, idx: usize) {
        let characters = self.lines[idx].characters.clone();
        for id in characters {
            let restore_speed = 40.0 / ctx.rng.randint(20, 40) as f64;
            ctx.terminal.arena[id.0 as usize]
                .motion
                .paths
                .get_mut("restore")
                .must_val()
                .speed = restore_speed;
            ctx.activate_path(self, id, "restore");
        }
    }

    /// Line.activate_path.
    pub(super) fn line_activate_path(&mut self, ctx: &mut EngineCtx, idx: usize, path_id: &str) {
        let characters = self.lines[idx].characters.clone();
        for id in characters {
            ctx.activate_path(self, id, path_id);
        }
    }

    /// Line.line_movement_complete.
    pub(super) fn line_movement_complete(&self, ctx: &EngineCtx, idx: usize) -> bool {
        self.lines[idx].characters.iter().all(|&id| {
            ctx.terminal.arena[id.0 as usize]
                .motion
                .movement_is_complete()
        })
    }

    pub(super) fn insert_line_characters(&self, ctx: &mut EngineCtx, idx: usize) {
        for &id in &self.lines[idx].characters {
            ctx.active_characters.insert(id);
        }
    }

    /// VHSTapeIterator.glitch_wave.
    pub(super) fn glitch_wave(&mut self, ctx: &mut EngineCtx) {
        // Python falsy check: None (0 is unreachable after the max(2, ...) clamp
        // but kept falsy for exactness).
        if matches!(self.active_glitch_wave_top, None | Some(0)) {
            if ctx.terminal.canvas.text_height >= 3 {
                // choose a wave top index in the top half of the canvas or at least 3 rows up
                let lower = std::cmp::max(
                    3,
                    round_half_even(ctx.terminal.canvas.text_height as f64 * 0.5),
                );
                self.active_glitch_wave_top = Some(
                    ctx.terminal.canvas.text_bottom
                        + ctx.rng.randint(lower, ctx.terminal.canvas.text_height),
                );
            } else {
                // not enough room for a wave
                return;
            }
        }

        // if all lines have completed movement, proceed to move/restore wave
        let all_complete = self
            .active_glitch_wave_lines
            .iter()
            .all(|&idx| self.line_movement_complete(ctx, idx));
        if all_complete {
            if !self.active_glitch_wave_lines.is_empty() {
                // only move 30% of the time — the OUTER conditional's condition is
                // evaluated first (second textual random() call in the source)
                let should_move = ctx.rng.random() < 0.3;
                let wave_top_delta = if should_move {
                    if ctx.rng.random() < 0.3 { 1 } else { -1 }
                } else {
                    0
                };
                let mut top = self.active_glitch_wave_top.must_val() + wave_top_delta;
                // clamp wave top to canvas
                top = std::cmp::max(2, std::cmp::min(top, ctx.terminal.canvas.text_top));
                self.active_glitch_wave_top = Some(top);
            }
            // get the lines for the wave
            let wave_top = self.active_glitch_wave_top.must_val();
            let mut new_wave_lines: Vec<usize> = Vec::new();
            for line_index in (wave_top - 2)..=wave_top {
                let adjusted_line_index = line_index - (ctx.terminal.canvas.text_bottom - 1);
                if adjusted_line_index >= 0 && (adjusted_line_index as usize) < self.lines.len() {
                    new_wave_lines.push(adjusted_line_index as usize);
                }
            }

            // restore any lines that are no longer part of the wave
            let old_wave_lines = std::mem::take(&mut self.active_glitch_wave_lines);
            for &idx in &old_wave_lines {
                if !new_wave_lines.contains(&idx) {
                    self.line_restore(ctx, idx);
                    self.insert_line_characters(ctx, idx);
                }
            }
            self.active_glitch_wave_lines = new_wave_lines;

            if wave_top < ctx.terminal.canvas.text_bottom + 2 {
                // wave at bottom, restore lines
                let wave_lines = std::mem::take(&mut self.active_glitch_wave_lines);
                for &idx in &wave_lines {
                    self.line_restore(ctx, idx);
                    self.insert_line_characters(ctx, idx);
                }
                self.active_glitch_wave_top = None;
            } else {
                let path_ids = ["glitch_wave_mid", "glitch_wave_end", "glitch_wave_mid"];
                let wave_lines = self.active_glitch_wave_lines.clone();
                for (idx, path_id) in wave_lines.iter().zip(path_ids.iter()) {
                    self.line_activate_path(ctx, *idx, path_id);
                    self.insert_line_characters(ctx, *idx);
                }
            }
        }
    }
}
impl EffectHooks for VhsTape {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
