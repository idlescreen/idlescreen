// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharId, CharacterFilter, CharacterGroup, ColorPair, Effect, EngineCtx, EngineError,
    ExistingColorHandling, Gradient, Overflow, OverflowConfig, Row, emit_frame, floor_div,
    hex_color,
};

impl Effect for Overflow {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let final_gradient = Gradient::new(
            &self.config.final_gradient_stops,
            &self.config.final_gradient_steps,
            false,
            false,
        )
        .map_err(EngineError::Other)?;
        let final_gradient_mapping = final_gradient
            .build_coordinate_color_mapping(
                ctx.terminal.canvas.text_bottom,
                ctx.terminal.canvas.text_top,
                ctx.terminal.canvas.text_left,
                ctx.terminal.canvas.text_right,
                self.config.final_gradient_direction,
            )
            .map_err(EngineError::Other)?;
        let fills_filter = CharacterFilter {
            inner_fill_chars: true,
            outer_fill_chars: true,
            ..Default::default()
        };
        let characters = ctx.terminal.get_characters(
            &mut ctx.rng,
            fills_filter,
            ttfx::engine::terminal::CharacterSort::TopToBottomLeftToRight,
        );
        for &id in &characters {
            let coord = ctx.terminal.arena[id.0 as usize].input_coord;
            let color = final_gradient_mapping
                .get(&coord)
                .copied()
                .unwrap_or_else(|| hex_color("000000"));
            self.character_final_color_map.insert(id, color);
        }
        let (lower_range, upper_range) = self.config.overflow_cycles_range;
        let mut rows = ctx
            .terminal
            .get_characters_grouped(CharacterFilter::default(), CharacterGroup::RowTopToBottom);
        if upper_range > 0 {
            for _ in 0..ctx.rng.randint(lower_range, upper_range) {
                ctx.rng.shuffle(&mut rows);
                for row in &rows {
                    let mut copied_characters: Vec<CharId> = Vec::new();
                    // copy the character attributes to new characters
                    for &id in row {
                        let (
                            symbol,
                            coord,
                            fg_seq,
                            bg_seq,
                            no_color,
                            use_xterm,
                            input_fg,
                            input_bg,
                        ) = {
                            let ch = &ctx.terminal.arena[id.0 as usize];
                            (
                                ch.input_symbol.clone(),
                                ch.input_coord,
                                ch.input_ansi_fg_sequence.clone(),
                                ch.input_ansi_bg_sequence.clone(),
                                ch.animation.no_color,
                                ch.animation.use_xterm_colors,
                                ch.animation.input_fg_color,
                                ch.animation.input_bg_color,
                            )
                        };
                        let copy_id = ctx.terminal.add_character(&symbol, coord);
                        let copy = &mut ctx.terminal.arena[copy_id.0 as usize];
                        copy.animation.existing_color_handling =
                            ctx.terminal.config.existing_color_handling;
                        copy.uses_input_preexisting_colors = true;
                        copy.input_ansi_fg_sequence = fg_seq;
                        copy.input_ansi_bg_sequence = bg_seq;
                        copy.animation.no_color = no_color;
                        copy.animation.use_xterm_colors = use_xterm;
                        copy.animation.input_fg_color = input_fg;
                        copy.animation.input_bg_color = input_bg;
                        copied_characters.push(copy_id);
                    }
                    self.pending_rows.push_back(Row {
                        characters: copied_characters,
                        final_: false,
                    });
                }
            }
        }
        // add rows in correct order to the end of pending_rows
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        for row in ctx
            .terminal
            .get_characters_grouped(fills_filter, CharacterGroup::RowTopToBottom)
        {
            for &id in &row {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let current_symbol = ch.animation.current_character_visual.symbol.clone();
                let input_symbol = ch.input_symbol.clone();
                let uses_pre = ch.uses_input_preexisting_colors;
                if dynamic {
                    let input_fg = ch.animation.input_fg_color;
                    let input_bg = ch.animation.input_bg_color;
                    if input_fg.is_some() || input_bg.is_some() {
                        ch.animation.set_appearance(
                            &input_symbol,
                            uses_pre,
                            Some(&current_symbol),
                            Some(ColorPair::new(input_fg, input_bg)),
                        );
                    } else {
                        ch.animation.set_appearance(
                            &input_symbol,
                            uses_pre,
                            Some(&current_symbol),
                            Some(ColorPair::default()),
                        );
                    }
                } else {
                    let final_color = *self
                        .character_final_color_map
                        .get(&id)
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    ch.animation.set_appearance(
                        &input_symbol,
                        uses_pre,
                        Some(&current_symbol),
                        Some(ColorPair::new(Some(final_color), None)),
                    );
                }
            }
            self.pending_rows.push_back(Row {
                characters: row,
                final_: true,
            });
        }
        self.delay = 0;
        let steps = std::cmp::max(
            floor_div(
                ctx.terminal.canvas.top,
                std::cmp::max(1, self.config.overflow_gradient_stops.len() as i64 - 1),
            ),
            1,
        );
        self.overflow_gradient = Some(
            Gradient::with_steps(&self.config.overflow_gradient_stops, steps, false)
                .map_err(EngineError::Other)?,
        );
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.pending_rows.is_empty() {
            if self.delay == 0 {
                let spectrum = &self.overflow_gradient.as_ref()?.spectrum;
                for _ in 0..ctx.rng.randint(1, self.config.overflow_speed) {
                    if !self.pending_rows.is_empty() {
                        for row in &self.active_rows {
                            row.move_up(ctx);
                            if !row.final_ {
                                let head_row = ctx.terminal.arena[row.characters[0].0 as usize]
                                    .motion
                                    .current_coord
                                    .row;
                                let index =
                                    head_row.clamp(0, (spectrum.len() as i64 - 1).max(0)) as usize;
                                row.set_color(ctx, Some(spectrum[index]), None);
                            }
                        }
                        let next_row = self.pending_rows.pop_front()?;
                        next_row.setup(ctx);
                        next_row.move_up(ctx);
                        if !next_row.final_ {
                            next_row.set_color(ctx, Some(spectrum[0]), None);
                        }
                        for &id in &next_row.characters {
                            ctx.terminal.set_character_visibility(id, true);
                        }
                        self.active_rows.push(next_row);
                    }
                }
                self.delay = ctx.rng.randint(0, 3);
            } else {
                self.delay -= 1;
            }
            let canvas_top = ctx.terminal.canvas.top;
            let arena = &ctx.terminal.arena;
            self.active_rows.retain(|row| {
                arena[row.characters[0].0 as usize].motion.current_coord.row <= canvas_top
            });
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Overflow {
    fn default() -> Self {
        Self::new(OverflowConfig::default())
    }
}
