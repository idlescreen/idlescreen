// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for ColorShift {
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
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for &id in &characters {
            let input_coord = ctx.terminal.arena[id.0 as usize].input_coord;
            self.character_final_color_map.insert(
                id,
                final_gradient_mapping.get(&input_coord).must_val().clone(),
            );
        }
        let gradient = Gradient::new(
            &self.config.gradient_stops,
            &self.config.gradient_steps,
            false,
            !self.config.no_loop,
        )
        .map_err(EngineError::Other)?;
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            ctx.terminal.set_character_visibility(id, true);
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
            let colors: Vec<Color> = if self.config.no_travel {
                gradient.spectrum.clone()
            } else {
                let direction_index = match self.config.travel_direction {
                    GradientDirection::Horizontal => {
                        input_coord.column as f64 / ctx.terminal.canvas.right as f64
                    }
                    GradientDirection::Vertical => {
                        input_coord.row as f64 / ctx.terminal.canvas.top as f64
                    }
                    GradientDirection::Diagonal => {
                        (input_coord.row + input_coord.column) as f64
                            / (ctx.terminal.canvas.right + ctx.terminal.canvas.top) as f64
                    }
                    GradientDirection::Radial => geometry::find_normalized_distance_from_center(
                        ctx.terminal.canvas.text_bottom,
                        ctx.terminal.canvas.text_top,
                        ctx.terminal.canvas.text_left,
                        ctx.terminal.canvas.text_right,
                        input_coord,
                    )
                    .map_err(EngineError::Other)?,
                };
                // int() truncation
                let mut shift_distance = (gradient.spectrum.len() as f64 * direction_index) as i64;
                if self.config.reverse_travel_direction {
                    shift_distance *= -1;
                }
                // Python slicing: spectrum[shift:] + spectrum[:shift], negative shifts wrap
                let len = gradient.spectrum.len() as i64;
                let k = if shift_distance < 0 {
                    (len + shift_distance).max(0) as usize
                } else {
                    shift_distance.min(len) as usize
                };
                let mut rotated = gradient.spectrum[k..].to_vec();
                rotated.extend_from_slice(&gradient.spectrum[..k]);
                rotated
            };
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .new_scene(false, None, None, "gradient", uses_pre);
                let scene = ch.animation.scenes.get_mut("gradient").must_val();
                for color in &colors {
                    scene
                        .add_frame(
                            &input_symbol,
                            self.config.gradient_frames,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(color.clone()), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                ch.animation
                    .new_scene(false, None, None, "final_gradient", uses_pre);
            }
            let last_color = colors.last().must_val().clone();
            if dynamic {
                let fg_gradient = match &input_fg {
                    Some(c) => Some(
                        Gradient::with_steps(&[last_color.clone(), c.clone()], 8, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let bg_gradient = match &input_bg {
                    Some(c) => Some(
                        Gradient::with_steps(&[last_color.clone(), c.clone()], 8, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene = ch.animation.scenes.get_mut("final_gradient").must_val();
                if fg_gradient.is_some() || bg_gradient.is_some() {
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            self.config.gradient_frames,
                            fg_gradient.as_ref(),
                            bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    scene
                        .add_frame(
                            &input_symbol,
                            self.config.gradient_frames,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            } else {
                let final_color = self.character_final_color_map.get(&id).must_val().clone();
                let final_scene_gradient =
                    Gradient::with_steps(&[last_color, final_color], 8, false)
                        .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene = ch.animation.scenes.get_mut("final_gradient").must_val();
                for color in &final_scene_gradient.spectrum {
                    scene
                        .add_frame(
                            &input_symbol,
                            self.config.gradient_frames,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(color.clone()), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }
            ctx.activate_scene(self, id, "gradient");
            ctx.active_characters.insert(id);
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene("gradient".to_string()),
                EventAction::Callback(EffectCallback {
                    id: 0,
                    args: Vec::new(),
                }),
            )
            .map_err(EngineError::Other)?;
        }
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !ctx.active_characters.is_empty() {
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}
impl Default for ColorShiftConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for ColorShift {
    fn default() -> Self {
        Self::new(ColorShiftConfig::default())
    }
}
