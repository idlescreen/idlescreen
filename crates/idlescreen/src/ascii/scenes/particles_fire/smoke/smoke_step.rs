// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    BreadthFirst, CallerKey, CharacterFilter, CharacterSort, Color, ColorPair, Effect, EngineCtx,
    EngineError, Event, EventAction, ExistingColorHandling, Gradient, PrimsWeighted, Smoke,
    SmokeConfig, VisualParams, emit_frame, hex_color,
};

impl Effect for Smoke {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // SmokeIterator.__init__ order: PrimsWeighted (random starting coord +
        // per-character weights), then the BreadthFirst starting character
        // (random coord lookup; a failed lookup falls back to BreadthFirst's
        // own draw, exactly like Python's `starting_char or ...`).
        let limit_to_text_boundary = !self.config.use_whole_canvas;
        let mut gen_alg =
            PrimsWeighted::new(ctx, None, limit_to_text_boundary).map_err(EngineError::Other)?;
        let fill_start_coord =
            ctx.terminal
                .canvas
                .random_coord(&mut ctx.rng, false, limit_to_text_boundary);
        let fill_start_char = ctx.terminal.get_character_by_input_coord(fill_start_coord);
        let fill_alg = BreadthFirst::new(ctx, fill_start_char, limit_to_text_boundary)
            .map_err(EngineError::Other)?;

        // SmokeIterator.build()
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
        let blk = hex_color("000000");
        // Gradient(*smoke_gradient_stops, *final_gradient_stops[::-1], steps=(3, 4))
        let smoke_gradient_colors: Vec<Color> = self
            .config
            .smoke_gradient_stops
            .iter()
            .chain(self.config.final_gradient_stops.iter().rev())
            .copied()
            .collect();
        let smoke_gradient = Gradient::new(&smoke_gradient_colors, &[3, 4], false, false)
            .map_err(EngineError::Other)?;

        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter {
                input_chars: true,
                inner_fill_chars: true,
                outer_fill_chars: true,
                added_chars: false,
            };
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for &id in &characters {
            ctx.terminal.set_character_visibility(id, true);
            let (input_coord, input_symbol, input_fg, input_bg, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.animation.input_fg_color,
                    ch.animation.input_bg_color,
                    ch.uses_input_preexisting_colors,
                )
            };
            let base_colors = if dynamic {
                self.character_final_color_map
                    .insert(id, ColorPair::new(input_fg, input_bg));
                ColorPair::new(Some(blk), None)
            } else {
                self.character_final_color_map.insert(
                    id,
                    ColorPair::new(
                        Some(
                            final_gradient_mapping
                                .get(&input_coord)
                                .copied()
                                .unwrap_or(blk),
                        ),
                        None,
                    ),
                );
                ColorPair::new(Some(self.config.starting_color), None)
            };
            let paint_chars = [input_symbol.clone()];
            let paint_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "paint", uses_pre)
            };
            if dynamic {
                let colors = *self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&paint_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        &input_symbol,
                        5,
                        VisualParams {
                            colors: Some(colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            } else {
                let final_fg_color = self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .fg_color
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                // Gradient(*final_gradient_stops, final_fg_color, steps=5)
                let paint_stops: Vec<Color> = self
                    .config
                    .final_gradient_stops
                    .iter()
                    .copied()
                    .chain(std::iter::once(final_fg_color))
                    .collect();
                let paint_gradient =
                    Gradient::with_steps(&paint_stops, 5, false).map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&paint_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .apply_gradient_to_symbols(&paint_chars, 5, Some(&paint_gradient), None)
                    .map_err(EngineError::Other)?;
            }

            let smoke_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "smoke", uses_pre)
            };
            if dynamic {
                let colors = *self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene = ch
                    .animation
                    .scenes
                    .get_mut(&smoke_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                for smoke_symbol in &self.config.smoke_symbols {
                    scene
                        .add_frame(
                            smoke_symbol,
                            10,
                            VisualParams {
                                colors: Some(colors),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            } else {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&smoke_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .apply_gradient_to_symbols(
                        &self.config.smoke_symbols,
                        3,
                        Some(&smoke_gradient),
                        None,
                    )
                    .map_err(EngineError::Other)?;
            }
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene(smoke_scn),
                EventAction::ActivateScene(paint_scn),
            )
            .map_err(EngineError::Other)?;
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.set_appearance(
                    &input_symbol,
                    uses_pre,
                    Some(&input_symbol),
                    Some(base_colors),
                );
            }
        }

        while !gen_alg.complete {
            gen_alg.step(ctx);
        }

        // trigger effects on starting char since it will not be 'explored'
        let starting_char = fill_alg.starting_char;
        ctx.activate_scene(self, starting_char, "smoke");
        ctx.active_characters.insert(starting_char);

        self.fill_alg = Some(fill_alg);
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        let mut fill_alg = self.fill_alg.take()?;
        let result = if !fill_alg.complete || !ctx.active_characters.is_empty() {
            if !fill_alg.complete {
                fill_alg.step(ctx);
                for i in 0..fill_alg.explored_last_step.len() {
                    let id = fill_alg.explored_last_step[i];
                    ctx.activate_scene(self, id, "smoke");
                    ctx.active_characters.insert(id);
                }
            }
            ctx.update(self);
            Some(emit_frame(ctx))
        } else {
            None
        };
        self.fill_alg = Some(fill_alg);
        result
    }
}

impl Default for Smoke {
    fn default() -> Self {
        Self::new(SmokeConfig::default())
    }
}
