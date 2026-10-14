// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl EffectHooks for Thunderstorm {
    fn dispatch_callback(
        &mut self,
        ctx: &mut EngineCtx,
        character: CharId,
        callback: &EffectCallback,
    ) {
        match callback.id {
            CB_FADE_COMPLETE => {
                self.phase = Phase::Storm;
                self.storm_start_time = ctx.clock.now_monotonic();
            }
            CB_HIDE_CHARACTER => ctx.terminal.set_character_visibility(character, false),
            CB_MAKE_CHAR_GLOW => {
                let coord = ctx.terminal.arena[character.0 as usize]
                    .motion
                    .current_coord;
                if let Some(input_char) = ctx.terminal.get_character_by_input_coord(coord) {
                    if ctx.terminal.arena[input_char.0 as usize].is_visible {
                        ctx.activate_scene(self, input_char, "glow");
                        self.pending_glow_chars.push(input_char);
                    }
                }
            }
            CB_RETURN_STRIKE_TO_POOL => self.available_strike_chars.push(character),
            CB_SET_STRIKE_IN_PROGRESS_FALSE => self.strike_in_progress = false,
            CB_RECLAIM_RAIN => self.rain_pool.reclaim(ctx, character, true, true),
            CB_RECLAIM_SPARK => self.spark_pool.reclaim(ctx, character, true, true),
            _ => {}
        }
    }
}
impl Effect for Thunderstorm {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // __init__ preamble (effect_thunderstorm.py:188-204): pools are
        // preallocated before build() runs, and the storm clock is read once.
        self.rain_pool
            .preallocate(ctx, 50, initialize_raindrop)
            .map_err(EngineError::Other)?;
        self.spark_gradient = Gradient::with_steps(
            &[
                self.config.spark_glow_color.clone(),
                ctx.terminal.config.terminal_background_color.clone(),
            ],
            7,
            false,
        )
        .map_err(EngineError::Other)?;
        {
            let spark_gradient = self.spark_gradient.clone();
            let spark_glow_time = self.config.spark_glow_time;
            self.spark_pool
                .preallocate(ctx, 200, |ctx, particle| {
                    initialize_spark(ctx, particle, &spark_gradient, spark_glow_time)
                })
                .map_err(EngineError::Other)?;
        }
        self.storm_start_time = ctx.clock.now_monotonic();

        // build() body
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
        self.build_strike_characters(ctx, 200);

        // setup scenes on text characters
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let dynamic_neutral_gray = Color::from_hex("808080").must_val();
        let all_chars = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for &id in &all_chars {
            let (input_symbol, input_coord, uses_pre, input_fg, input_bg) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_symbol.clone(),
                    ch.input_coord,
                    ch.uses_input_preexisting_colors,
                    ch.animation.input_fg_color.clone(),
                    ch.animation.input_bg_color.clone(),
                )
            };
            let (visible_colors, restore_colors) = if dynamic {
                let visible = ColorPair::new(
                    Some(
                        input_fg
                            .clone()
                            .unwrap_or_else(|| dynamic_neutral_gray.clone()),
                    ),
                    input_bg.clone(),
                );
                let restore = ColorPair::new(input_fg.clone(), input_bg.clone());
                (visible, restore)
            } else {
                let visible = ColorPair::new(
                    Some(final_gradient_mapping.get(&input_coord).must_val().clone()),
                    None,
                );
                (visible.clone(), visible)
            };
            let storm_colors = adjust_color_pair_brightness(&visible_colors, 0.5);
            // upstream stores visible/storm/restore in three maps that are
            // never read back; not stored here.

            // post-strike glow and cool scene
            let glow_fg_gradient = Gradient::with_steps(
                &[
                    self.config.glowing_text_color.clone(),
                    storm_colors.fg_color.clone().must_val(),
                ],
                7,
                false,
            )
            .map_err(EngineError::Other)?;
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let glow_scn = ch.animation.new_scene(false, None, None, "glow", uses_pre);
                let scene = ch.animation.scenes.get_mut(&glow_scn).must_val();
                for color in &glow_fg_gradient.spectrum {
                    scene
                        .add_frame(
                            &input_symbol,
                            self.config.text_glow_time,
                            VisualParams {
                                colors: Some(ColorPair::new(
                                    Some(color.clone()),
                                    storm_colors.bg_color.clone(),
                                )),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                if dynamic {
                    scene
                        .add_frame(
                            &input_symbol,
                            self.config.text_glow_time,
                            VisualParams {
                                colors: Some(storm_colors.clone()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }

            // fade before storm scene
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let fade_scn = ch.animation.new_scene(false, None, None, "fade", uses_pre);
                if dynamic {
                    let scene = ch.animation.scenes.get_mut(&fade_scn).must_val();
                    add_color_pair_gradient_frames(
                        scene,
                        &input_symbol,
                        &visible_colors,
                        &storm_colors,
                        7,
                        12,
                    )
                    .map_err(EngineError::Other)?;
                    scene
                        .add_frame(
                            &input_symbol,
                            12,
                            VisualParams {
                                colors: Some(storm_colors.clone()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    let fade_gradient = Gradient::with_steps(
                        &[
                            visible_colors.fg_color.clone().must_val(),
                            storm_colors.fg_color.clone().must_val(),
                        ],
                        7,
                        false,
                    )
                    .map_err(EngineError::Other)?;
                    let scene = ch.animation.scenes.get_mut(&fade_scn).must_val();
                    for color in &fade_gradient.spectrum {
                        scene
                            .add_frame(
                                &input_symbol,
                                12,
                                VisualParams {
                                    colors: Some(ColorPair::new(Some(color.clone()), None)),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                }
            }

            self.setup_thunderstorm_unfade_flash(
                ctx,
                id,
                &input_symbol,
                uses_pre,
                dynamic,
                &visible_colors,
                &restore_colors,
                &storm_colors,
            )?;
        }

        // reference character callback: signals the pre-storm fade completed
        let reference_char = all_chars[0];
        ctx.register_event(
            reference_char,
            Event::SceneComplete,
            CallerKey::Scene("fade".to_string()),
            EventAction::Callback(EffectCallback {
                id: CB_FADE_COMPLETE,
                args: Vec::new(),
            }),
        )
        .map_err(EngineError::Other)?;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        self.step_scene(ctx)
    }
}
