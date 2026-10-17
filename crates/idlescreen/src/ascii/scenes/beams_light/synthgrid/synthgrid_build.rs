// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{Effect, EngineCtx, EngineError, Phase, SynthGrid, SynthGridConfig, emit_frame};

impl Effect for SynthGrid {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.build_scene(ctx)
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.pending_groups.is_empty()
            || !ctx.active_characters.is_empty()
            || self.phase != Phase::Complete
        {
            match self.phase {
                Phase::GridExpand => {
                    if self.grid_lines.iter().all(super::GridLine::is_extended) {
                        self.phase = Phase::AddChars;
                    } else {
                        let mut grid_lines = std::mem::take(&mut self.grid_lines);
                        for grid_line in &mut grid_lines {
                            if !grid_line.is_extended() {
                                grid_line.extend(ctx);
                            }
                        }
                        self.grid_lines = grid_lines;
                    }
                }
                Phase::AddChars => {
                    if !self.pending_groups.is_empty()
                        && (self.active_groups as f64)
                            < self.total_group_count as f64 * self.config.max_active_blocks
                    {
                        let (group_number, next_group) = self.pending_groups.remove(0);
                        for &ch in &next_group {
                            ctx.terminal.set_character_visibility(ch, true);
                            ctx.active_characters.insert(ch);
                            self.group_tracker[group_number as usize] += 1;
                        }
                    }
                    if self.pending_groups.is_empty()
                        && ctx.active_characters.is_empty()
                        && self.active_groups == 0
                    {
                        self.phase = Phase::Collapse;
                    }
                }
                Phase::Collapse => {
                    if self.grid_lines.iter().all(super::GridLine::is_collapsed) {
                        self.phase = Phase::Complete;
                    } else {
                        let mut grid_lines = std::mem::take(&mut self.grid_lines);
                        for grid_line in &mut grid_lines {
                            if !grid_line.is_collapsed() {
                                grid_line.collapse(ctx);
                            }
                        }
                        self.grid_lines = grid_lines;
                    }
                }
                Phase::Complete => {}
            }
            ctx.update(self);
            self.active_groups = 0;
            for &active_count in &self.group_tracker {
                if active_count != 0 {
                    self.active_groups += 1;
                }
            }
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for SynthGrid {
    fn default() -> Self {
        Self::new(SynthGridConfig::default())
    }
}
