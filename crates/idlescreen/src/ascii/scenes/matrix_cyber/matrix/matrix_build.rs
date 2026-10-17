// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{ColumnPhase, Effect, EngineCtx, EngineError, Matrix, MatrixConfig, Phase, emit_frame};

impl Effect for Matrix {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.build_scene(ctx)
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.phase == Phase::Rain || self.phase == Phase::Fill {
            if self.column_delay == 0 {
                if self.phase == Phase::Rain {
                    for _ in 0..ctx.rng.randint(1, 3) {
                        if !self.pending_columns.is_empty() {
                            self.active_columns.push(self.pending_columns.remove(0));
                        }
                    }
                } else {
                    while !self.pending_columns.is_empty() {
                        self.active_columns.push(self.pending_columns.remove(0));
                    }
                }
                self.column_delay = if self.phase == Phase::Rain {
                    ctx.rng.randint(
                        self.config.rain_column_delay_range.0,
                        self.config.rain_column_delay_range.1,
                    )
                } else {
                    1
                };
            } else {
                self.column_delay -= 1;
            }
            let active_snapshot = self.active_columns.clone();
            for column_index in active_snapshot {
                {
                    let (columns, config, rain_colors) =
                        (&mut self.columns, &self.config, &self.rain_colors);
                    columns[column_index].tick(ctx, config, rain_colors);
                }

                if self.columns[column_index].pending_characters.is_empty() {
                    if self.columns[column_index].phase == ColumnPhase::Fill
                        && !self.full_columns.contains(&column_index)
                    {
                        self.full_columns.push(column_index);
                    } else if self.columns[column_index].visible_characters.is_empty() {
                        let column_phase = self.column_phase_for();
                        let (columns, config) = (&mut self.columns, &self.config);
                        columns[column_index].setup_column(ctx, config, column_phase);
                        self.pending_columns.push(column_index);
                    }
                }
            }

            {
                let columns = &self.columns;
                self.active_columns
                    .retain(|&ci| !columns[ci].visible_characters.is_empty());
            }
            if self.phase == Phase::Fill
                && self.pending_columns.is_empty()
                && self.active_columns.iter().all(|&ci| {
                    self.columns[ci].pending_characters.is_empty()
                        && self.columns[ci].phase == ColumnPhase::Fill
                })
            {
                self.phase = Phase::Resolve;
                self.active_columns.clear();
            }

            // effect_matrix.py:549 — time.time() rain deadline check.
            if self.phase == Phase::Rain
                && self.config.rain_time > 0
                && ctx.clock.now_wall() - self.rain_start > self.config.rain_time as f64
            {
                self.rain_complete = true;
                self.phase = Phase::Fill;
                for &ci in &self.active_columns {
                    self.columns[ci].hold_time = 0;
                    self.columns[ci].column_drop_chance = 1.0;
                }
                let pending_snapshot = self.pending_columns.clone();
                for ci in pending_snapshot {
                    let (columns, config) = (&mut self.columns, &self.config);
                    columns[ci].setup_column(ctx, config, ColumnPhase::Fill);
                }
            }
        } else if self.phase == Phase::Resolve {
            let full_snapshot = self.full_columns.clone();
            for column_index in full_snapshot {
                {
                    let (columns, config, rain_colors) =
                        (&mut self.columns, &self.config, &self.rain_colors);
                    columns[column_index].tick(ctx, config, rain_colors);
                }
                if !self.columns[column_index].visible_characters.is_empty() {
                    if self.resolve_delay == 0 {
                        for _ in 0..ctx.rng.randint(1, 4) {
                            if !self.columns[column_index].visible_characters.is_empty() {
                                let next_char = self.columns[column_index].resolve_char(ctx);
                                let input_symbol = ctx.terminal.arena[next_char.0 as usize]
                                    .input_symbol
                                    .clone();
                                if input_symbol != " " || Self::has_input_colors(ctx, next_char) {
                                    ctx.activate_scene(self, next_char, "resolve");
                                    ctx.active_characters.insert(next_char);
                                } else {
                                    ctx.terminal.set_character_visibility(next_char, false);
                                }
                            }
                        }
                        self.resolve_delay = self.config.resolve_delay;
                    } else {
                        self.resolve_delay -= 1;
                    }
                }
            }
            {
                let columns = &self.columns;
                self.full_columns
                    .retain(|&ci| !columns[ci].visible_characters.is_empty());
            }
        }

        if !self.full_columns.is_empty()
            || !self.active_columns.is_empty()
            || !ctx.active_characters.is_empty()
            || !self.pending_columns.is_empty()
            || !self.rain_complete
        {
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        if !self.final_frame_shown {
            self.final_frame_shown = true;
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::new(MatrixConfig::default())
    }
}
