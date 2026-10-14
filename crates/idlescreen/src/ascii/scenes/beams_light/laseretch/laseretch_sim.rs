// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

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
                    ch.animation.input_fg_color.clone(),
                    ch.animation.input_bg_color.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            let (final_fg_color, final_bg_color, cool_gradient) = if dynamic {
                let pair = ColorPair::new(input_fg, input_bg);
                self.character_final_color_map.insert(id, pair.clone());
                let cool = Gradient::with_steps(&self.config.cool_gradient_stops, 8, false)
                    .map_err(EngineError::Other)?;
                (pair.fg_color, pair.bg_color, cool)
            } else {
                let mapped = final_gradient_mapping.get(&input_coord).must_val().clone();
                let pair = ColorPair::new(Some(mapped.clone()), None);
                self.character_final_color_map.insert(id, pair.clone());
                let mut stops = self.config.cool_gradient_stops.clone();
                stops.push(mapped);
                let cool = Gradient::with_steps(&stops, 8, false).map_err(EngineError::Other)?;
                (pair.fg_color, pair.bg_color, cool)
            };
            // gradients for the dynamic tail, built before borrowing the scene
            let cool_last = cool_gradient.spectrum.last().must_val().clone();
            let mut fg_gradient: Option<Gradient> = None;
            let mut bg_gradient: Option<Gradient> = None;
            let mut white_cooldown: Option<Gradient> = None;
            if dynamic {
                if final_fg_color.is_some() || final_bg_color.is_some() {
                    if let Some(fg) = &final_fg_color {
                        fg_gradient = Some(
                            Gradient::with_steps(&[cool_last.clone(), fg.clone()], 8, false)
                                .map_err(EngineError::Other)?,
                        );
                    }
                    if let Some(bg) = &final_bg_color {
                        bg_gradient = Some(
                            Gradient::with_steps(&[cool_last.clone(), bg.clone()], 8, false)
                                .map_err(EngineError::Other)?,
                        );
                    }
                } else {
                    white_cooldown = Some(
                        Gradient::with_steps(
                            &[cool_last.clone(), Color::from_hex("ffffff").must_val()],
                            8,
                            false,
                        )
                        .map_err(EngineError::Other)?,
                    );
                }
            }
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let spawn_scn = ch.animation.new_scene(false, None, None, "spawn", uses_pre);
                let scene = ch.animation.scenes.get_mut(&spawn_scn).must_val();
                scene
                    .add_frame(
                        "^",
                        3,
                        VisualParams {
                            colors: Some(ColorPair::new(
                                Some(Color::from_hex("ffe680").must_val()),
                                None,
                            )),
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
                                colors: Some(ColorPair::new(Some(color.clone()), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                if dynamic {
                    if final_fg_color.is_some() || final_bg_color.is_some() {
                        scene
                            .apply_gradient_to_symbols(
                                &[input_symbol.clone()],
                                3,
                                fg_gradient.as_ref(),
                                bg_gradient.as_ref(),
                            )
                            .map_err(EngineError::Other)?;
                    } else {
                        scene
                            .apply_gradient_to_symbols(
                                &[input_symbol.clone()],
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
                    if !self.pending_chars.is_empty() {
                        next_char = self.pending_chars.remove(0);
                    } else {
                        break;
                    }
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
        if !self.pending_chars.is_empty() {
            let beam_chars = self.laser.as_ref().must_val().beam_chars.clone();
            for id in beam_chars {
                ctx.active_characters.insert(id);
            }
        } else {
            self.laser_disable(ctx);
        }
        ctx.update(self);
        Some(emit_frame(ctx))
    }
}
impl Default for LaserEtchConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for LaserEtch {
    fn default() -> Self {
        Self::new(LaserEtchConfig::default())
    }
}
