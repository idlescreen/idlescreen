// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for Waves {
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
        let wave_gradient = Gradient::new(
            &self.config.wave_gradient_stops,
            &self.config.wave_gradient_steps,
            false,
            false,
        )
        .map_err(EngineError::Other)?;

        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            let (input_fg, input_bg, input_coord, input_symbol, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.animation.input_fg_color.clone(),
                    ch.animation.input_bg_color.clone(),
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            let final_colors = if dynamic {
                ColorPair::new(input_fg, input_bg)
            } else {
                ColorPair::new(
                    Some(final_gradient_mapping.get(&input_coord).must_val().clone()),
                    None,
                )
            };
            self.character_final_color_map
                .insert(id, final_colors.clone());

            let wave_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene_id = ch.animation.new_scene(
                    false,
                    None,
                    Some(self.config.wave_easing),
                    "",
                    uses_pre,
                );
                let scene = ch.animation.scenes.get_mut(&scene_id).must_val();
                for _ in 0..self.config.wave_count {
                    scene
                        .apply_gradient_to_symbols(
                            &self.config.wave_symbols,
                            self.config.wave_length,
                            Some(&wave_gradient),
                            None,
                        )
                        .map_err(EngineError::Other)?;
                }
                scene_id
            };
            let final_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "", uses_pre)
            };
            if dynamic {
                let final_fg_color = final_colors.fg_color.clone();
                let final_bg_color = final_colors.bg_color.clone();
                if final_fg_color.is_none() && final_bg_color.is_none() {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation
                        .scenes
                        .get_mut(&final_scn)
                        .must_val()
                        .add_frame(
                            &input_symbol,
                            10,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    let fg_gradient = match &final_fg_color {
                        Some(c) => Some(
                            Gradient::new(
                                &[wave_gradient.spectrum.last().must_val().clone(), c.clone()],
                                &self.config.final_gradient_steps,
                                false,
                                false,
                            )
                            .map_err(EngineError::Other)?,
                        ),
                        None => None,
                    };
                    let bg_gradient = match &final_bg_color {
                        Some(c) => Some(
                            Gradient::new(
                                &[wave_gradient.spectrum.last().must_val().clone(), c.clone()],
                                &self.config.final_gradient_steps,
                                false,
                                false,
                            )
                            .map_err(EngineError::Other)?,
                        ),
                        None => None,
                    };
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let scene = ch.animation.scenes.get_mut(&final_scn).must_val();
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            10,
                            fg_gradient.as_ref(),
                            bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                    if final_fg_color.is_none() {
                        scene
                            .add_frame(
                                &input_symbol,
                                10,
                                VisualParams {
                                    colors: Some(ColorPair::new(None, final_bg_color.clone())),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                }
            } else {
                let final_fg_color = final_colors.fg_color.clone().must_val();
                let final_scene_gradient = Gradient::new(
                    &[
                        wave_gradient.spectrum.last().must_val().clone(),
                        final_fg_color,
                    ],
                    &self.config.final_gradient_steps,
                    false,
                    false,
                )
                .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene = ch.animation.scenes.get_mut(&final_scn).must_val();
                for step in &final_scene_gradient.spectrum {
                    scene
                        .add_frame(
                            &input_symbol,
                            10,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(step.clone()), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene(wave_scn.clone()),
                EventAction::ActivateScene(final_scn),
            )
            .map_err(EngineError::Other)?;
            ctx.activate_scene(self, id, &wave_scn);
            if dynamic {
                let final_colors = self.character_final_color_map.get(&id).must_val().clone();
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.set_appearance(
                    &input_symbol,
                    uses_pre,
                    Some(&input_symbol),
                    Some(final_colors),
                );
            }
        }

        for column in ctx
            .terminal
            .get_characters_grouped(CharacterFilter::default(), self.config.wave_direction)
        {
            self.pending_columns.push(column);
        }
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.pending_columns.is_empty() || !ctx.active_characters.is_empty() {
            if !self.pending_columns.is_empty() {
                let next_column = self.pending_columns.remove(0);
                for id in next_column {
                    ctx.terminal.set_character_visibility(id, true);
                    ctx.active_characters.insert(id);
                }
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}
impl Default for WavesConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Waves {
    fn default() -> Self {
        Self::new(WavesConfig::default())
    }
}
