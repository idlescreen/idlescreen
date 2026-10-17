// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharacterFilter, CharacterGroup, CharacterSort, ColorPair, Effect, EngineCtx, EngineError,
    ExistingColorHandling, Gradient, Line, Phase, VhsTape, VhsTapeConfig, emit_frame, hex_color,
};

impl Effect for VhsTape {
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
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            let ch = &ctx.terminal.arena[id.0 as usize];
            if dynamic {
                let input_fg = ch.animation.input_fg_color;
                let input_bg = ch.animation.input_bg_color;
                // DYNAMIC_NEUTRAL_GRAY
                let stable_fg = input_fg.or_else(|| Some(hex_color("808080")));
                self.character_stable_color_map
                    .insert(id, ColorPair::new(stable_fg, input_bg));
                self.character_final_color_map
                    .insert(id, ColorPair::new(input_fg, input_bg));
            } else {
                let gradient_color = *final_gradient_mapping
                    .get(&ch.input_coord)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let stable_colors = ColorPair::new(Some(gradient_color), None);
                self.character_stable_color_map.insert(id, stable_colors);
                self.character_final_color_map.insert(id, stable_colors);
            }
        }
        let rows = ctx
            .terminal
            .get_characters_grouped(CharacterFilter::default(), CharacterGroup::RowBottomToTop);
        for characters in rows {
            self.build_line_effects(ctx, &characters)?;
            self.lines.push(Line { characters });
        }
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            ctx.terminal.set_character_visibility(id, true);
            ctx.activate_scene(self, id, "base");
        }
        self.glitching_steps_elapsed = 0;
        self.phase = Phase::Glitching;
        self.to_redraw = (0..self.lines.len()).collect();
        self.redrawing = false;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.phase == Phase::Complete && ctx.active_characters.is_empty() {
            return None;
        }
        match self.phase {
            Phase::Glitching => {
                // Check if all active glitch wave lines have completed their movement, if so move the wave
                if self.active_glitch_wave_lines.is_empty()
                    || self
                        .active_glitch_wave_lines
                        .iter()
                        .all(|&idx| self.line_movement_complete(ctx, idx))
                {
                    self.glitch_wave(ctx);
                }
                // Remove completed glitch lines from active glitch lines
                let glitch_lines = std::mem::take(&mut self.active_glitch_lines);
                self.active_glitch_lines = glitch_lines
                    .into_iter()
                    .filter(|&idx| !self.line_movement_complete(ctx, idx))
                    .collect();
                // Randomly add new glitch lines
                if ctx.rng.random() < self.config.glitch_line_chance
                    && self.active_glitch_lines.len() < 3
                {
                    let glitch_line = ctx.rng.choice_index(self.lines.len());
                    if !self.active_glitch_wave_lines.contains(&glitch_line)
                        && !self.active_glitch_lines.contains(&glitch_line)
                    {
                        let hold_time = ctx.rng.randint(20, 75);
                        self.line_set_hold_time(ctx, glitch_line, hold_time);
                        self.active_glitch_lines.push(glitch_line);
                        self.line_glitch(ctx, glitch_line, false);
                        self.insert_line_characters(ctx, glitch_line);
                    }
                }
                // Randomly add noise to all lines
                if ctx.rng.random() < self.config.noise_chance {
                    for idx in 0..self.lines.len() {
                        self.line_snow(ctx, idx);
                        if !self.active_glitch_wave_lines.contains(&idx)
                            && !self.active_glitch_lines.contains(&idx)
                        {
                            self.insert_line_characters(ctx, idx);
                        }
                    }
                }
                self.glitching_steps_elapsed += 1;
                // Check if glitching time has reached the total glitch time
                if self.glitching_steps_elapsed >= self.config.total_glitch_time {
                    // Restore glitch wave lines
                    let wave_lines = self.active_glitch_wave_lines.clone();
                    for idx in wave_lines {
                        self.line_restore(ctx, idx);
                    }
                    // Restore glitch lines
                    let glitch_lines = self.active_glitch_lines.clone();
                    for idx in glitch_lines {
                        self.line_restore(ctx, idx);
                    }
                    self.phase = Phase::Noise;
                }
            }
            Phase::Noise => {
                // Activate final snow animation for all characters
                if ctx.active_characters.is_empty() {
                    let characters = {
                        let filter = CharacterFilter::default();
                        ctx.terminal.get_characters(
                            &mut ctx.rng,
                            filter,
                            CharacterSort::TopToBottomLeftToRight,
                        )
                    };
                    for id in characters {
                        ctx.activate_scene(self, id, "final_snow");
                        ctx.active_characters.insert(id);
                    }
                    self.phase = Phase::Redraw;
                }
            }
            Phase::Redraw => {
                // Redraw lines one by one
                if self.redrawing || ctx.active_characters.is_empty() {
                    self.redrawing = true;
                    if let Some(next_line) = self.to_redraw.pop() {
                        let characters = self.lines[next_line].characters.clone();
                        for id in characters {
                            ctx.activate_scene(self, id, "final_redraw");
                            ctx.active_characters.insert(id);
                        }
                    } else {
                        self.phase = Phase::Complete;
                    }
                }
            }
            Phase::Complete => {}
        }
        ctx.update(self);
        Some(emit_frame(ctx))
    }
}

impl Default for VhsTape {
    fn default() -> Self {
        Self::new(VhsTapeConfig::default())
    }
}
