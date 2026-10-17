// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharacterFilter, CharacterSort, ColorPair, Effect, EngineCtx, EngineError, EtchPattern,
    ExistingColorHandling, Gradient, LaserEtch, LaserEtchConfig, RecursiveBacktracker,
    VisualParams, emit_frame, has_input_colors, hex_color,
};

impl Effect for LaserEtch {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // LaserEtchIterator.build
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
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
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
            let (final_fg_color, final_bg_color, cool_gradient) = if dynamic {
                let pair = ColorPair::new(input_fg, input_bg);
                self.character_final_color_map.insert(id, pair);
                let cool = Gradient::with_steps(&self.config.cool_gradient_stops, 8, false)
                    .map_err(EngineError::Other)?;
                (pair.fg_color, pair.bg_color, cool)
            } else {
                let mapped = *final_gradient_mapping
                    .get(&input_coord)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let pair = ColorPair::new(Some(mapped), None);
                self.character_final_color_map.insert(id, pair);
                let mut stops = self.config.cool_gradient_stops.clone();
                stops.push(mapped);
                let cool = Gradient::with_steps(&stops, 8, false).map_err(EngineError::Other)?;
                (pair.fg_color, pair.bg_color, cool)
            };
            // gradients for the dynamic tail, built before borrowing the scene
            let cool_last = *cool_gradient
                .spectrum
                .last()
                .ok_or_else(|| EngineError::Other(String::new()))?;
            let mut fg_gradient: Option<Gradient> = None;
            let mut bg_gradient: Option<Gradient> = None;
            let mut white_cooldown: Option<Gradient> = None;
            if dynamic {
                if final_fg_color.is_some() || final_bg_color.is_some() {
                    if let Some(fg) = &final_fg_color {
                        fg_gradient = Some(
                            Gradient::with_steps(&[cool_last, *fg], 8, false)
                                .map_err(EngineError::Other)?,
                        );
                    }
                    if let Some(bg) = &final_bg_color {
                        bg_gradient = Some(
                            Gradient::with_steps(&[cool_last, *bg], 8, false)
                                .map_err(EngineError::Other)?,
                        );
                    }
                } else {
                    white_cooldown = Some(
                        Gradient::with_steps(&[cool_last, hex_color("ffffff")], 8, false)
                            .map_err(EngineError::Other)?,
                    );
                }
            }
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let spawn_scn = ch.animation.new_scene(false, None, None, "spawn", uses_pre);
                let scene = ch
                    .animation
                    .scenes
                    .get_mut(&spawn_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                scene
                    .add_frame(
                        "^",
                        3,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(hex_color("ffe680")), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                for color in &cool_gradient.spectrum {
                    scene
                        .add_frame(
                            &input_symbol,
                            3,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(*color), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                if dynamic {
                    if final_fg_color.is_some() || final_bg_color.is_some() {
                        scene
                            .apply_gradient_to_symbols(
                                std::slice::from_ref(&input_symbol),
                                3,
                                fg_gradient.as_ref(),
                                bg_gradient.as_ref(),
                            )
                            .map_err(EngineError::Other)?;
                    } else {
                        scene
                            .apply_gradient_to_symbols(
                                std::slice::from_ref(&input_symbol),
                                3,
                                white_cooldown.as_ref(),
                                None,
                            )
                            .map_err(EngineError::Other)?;
                        scene
                            .add_frame(
                                &input_symbol,
                                3,
                                VisualParams {
                                    colors: Some(ColorPair::default()),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                }
            }
            ctx.activate_scene(self, id, "spawn");
        }
        match self.config.etch_pattern {
            EtchPattern::Group(_) => {
                // Dead upstream branch — see module docs. pending_chars stays empty.
            }
            EtchPattern::Algorithm => {
                let mut algo =
                    RecursiveBacktracker::new(ctx, None, true).map_err(EngineError::Other)?;
                while !algo.complete {
                    algo.step(ctx);
                }
                self.pending_chars = algo.char_link_order;
            }
        }

        // LaserEtchIterator.__init__ tail
        self.char_delay = 0;
        let laser = self.make_laser(ctx)?;
        for &id in &laser.beam_chars {
            ctx.active_characters.insert(id);
        }
        self.laser = Some(laser);
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.pending_chars.is_empty() && ctx.active_characters.is_empty() {
            return None;
        }
        if self.char_delay == 0 {
            for _ in 0..self.config.etch_speed {
                if self.pending_chars.is_empty() {
                    break;
                }
                let mut next_char = self.pending_chars.remove(0);
                while ctx.terminal.arena[next_char.0 as usize].input_symbol == " "
                    && !has_input_colors(ctx, next_char)
                {
                    if self.pending_chars.is_empty() {
                        break;
                    }
                    next_char = self.pending_chars.remove(0);
                }
                ctx.terminal.set_character_visibility(next_char, true);
                ctx.active_characters.insert(next_char);
                let target = ctx.terminal.arena[next_char.0 as usize].input_coord;
                self.laser_reposition(ctx, target);
            }
            self.char_delay = self.config.etch_delay;
        } else {
            self.char_delay -= 1;
        }
        if self.pending_chars.is_empty() {
            self.laser_disable(ctx);
        } else {
            let beam_chars = self.laser.as_ref()?.beam_chars.clone();
            for id in beam_chars {
                ctx.active_characters.insert(id);
            }
        }
        ctx.update(self);
        Some(emit_frame(ctx))
    }
}

impl Default for LaserEtch {
    fn default() -> Self {
        Self::new(LaserEtchConfig::default())
    }
}
