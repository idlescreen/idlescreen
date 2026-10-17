// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharacterFilter, CharacterSort, Color, ColorPair, Easing, Effect, EngineCtx, EngineError,
    ExistingColorHandling, Gradient, SequenceEaser, Sweep, SweepConfig, VisualParams, emit_frame,
    hex_color,
};

impl Effect for Sweep {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let final_fg_gradient = Gradient::new(
            &self.config.final_gradient_stops,
            &self.config.final_gradient_steps,
            false,
            false,
        )
        .map_err(EngineError::Other)?;
        let final_gradient_mapping = final_fg_gradient
            .build_coordinate_color_mapping(
                ctx.terminal.canvas.text_bottom,
                ctx.terminal.canvas.text_top,
                ctx.terminal.canvas.text_left,
                ctx.terminal.canvas.text_right,
                self.config.final_gradient_direction,
            )
            .map_err(EngineError::Other)?;
        let shades_of_gray: Vec<Color> = ["A0A0A0", "808080", "404040", "202020", "101010"]
            .iter()
            .map(|hex| hex_color(hex))
            .collect();

        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        if dynamic {
            let characters = {
                let filter = CharacterFilter::default();
                ctx.terminal.get_characters(
                    &mut ctx.rng,
                    filter,
                    CharacterSort::TopToBottomLeftToRight,
                )
            };
            for id in characters {
                let ch = &ctx.terminal.arena[id.0 as usize];
                if let Some(fg) = &ch.animation.input_fg_color {
                    self.dynamic_second_sweep_palette.push(*fg);
                }
                if let Some(bg) = &ch.animation.input_bg_color {
                    self.dynamic_second_sweep_palette.push(*bg);
                }
            }
            if self.dynamic_second_sweep_palette.is_empty() {
                self.dynamic_second_sweep_palette
                    .clone_from(&final_fg_gradient.spectrum);
            }
        }

        let fills_filter = CharacterFilter {
            inner_fill_chars: true,
            outer_fill_chars: true,
            ..Default::default()
        };
        let characters = ctx.terminal.get_characters(
            &mut ctx.rng,
            fills_filter,
            CharacterSort::TopToBottomLeftToRight,
        );
        for id in characters {
            let (is_fill, input_fg, input_bg, input_coord, input_symbol, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.is_fill_character,
                    ch.animation.input_fg_color,
                    ch.animation.input_bg_color,
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            if !is_fill {
                let final_colors = if dynamic {
                    ColorPair::new(input_fg, input_bg)
                } else {
                    ColorPair::new(
                        Some(
                            *final_gradient_mapping
                                .get(&input_coord)
                                .ok_or_else(|| EngineError::Other(String::new()))?,
                        ),
                        None,
                    )
                };
                self.character_final_color_map.insert(id, final_colors);
            }
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .new_scene(false, None, None, "initial_sweep", uses_pre);
            }
            for symbol in &self.config.sweep_symbols {
                let color = *ctx.rng.choice(&shades_of_gray);
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("initial_sweep")
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        symbol,
                        5,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(color), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("initial_sweep")
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(hex_color("#808080")), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                ch.animation
                    .new_scene(false, None, None, "second_sweep", uses_pre);
            }
            for symbol in &self.config.sweep_symbols {
                let color = if dynamic {
                    *ctx.rng.choice(&self.dynamic_second_sweep_palette)
                } else {
                    *ctx.rng.choice(&final_fg_gradient.spectrum)
                };
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("second_sweep")
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        symbol,
                        5,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(color), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            let final_colors = if !is_fill {
                *self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
            } else if dynamic {
                ColorPair::default()
            } else {
                ColorPair::new(Some(hex_color("000000")), None)
            };
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("second_sweep")
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(final_colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
        }

        let groups_first_sweep = ctx
            .terminal
            .get_characters_grouped(fills_filter, self.config.first_sweep_direction);
        self.easer = Some(SequenceEaser::new(
            groups_first_sweep,
            Easing::InOutCirc,
            100,
        ));
        self.groups_second_sweep = ctx
            .terminal
            .get_characters_grouped(fills_filter, self.config.second_sweep_direction);
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !ctx.active_characters.is_empty() || !self.complete {
            let mut easer = self.easer.take()?;
            let step = easer.step();
            for group in step.added {
                for &id in group {
                    if self.first_phase {
                        ctx.terminal.set_character_visibility(id, true);
                    }
                    let scene_id = if self.first_phase {
                        "initial_sweep"
                    } else {
                        "second_sweep"
                    };
                    ctx.activate_scene(self, id, scene_id);
                }
                for &id in group {
                    ctx.active_characters.insert(id);
                }
            }
            let easer_complete = easer.is_complete();
            if easer_complete && self.first_phase {
                easer.sequence = std::mem::take(&mut self.groups_second_sweep);
                easer.reset();
                self.first_phase = false;
            } else if easer_complete && !self.first_phase {
                self.complete = true;
            }
            self.easer = Some(easer);
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Sweep {
    fn default() -> Self {
        Self::new(SweepConfig::default())
    }
}
