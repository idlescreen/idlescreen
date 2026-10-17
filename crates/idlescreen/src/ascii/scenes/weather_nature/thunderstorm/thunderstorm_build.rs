// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CB_FADE_COMPLETE, CallerKey, CharacterFilter, CharacterSort, ColorPair, Effect, EffectCallback,
    EngineCtx, EngineError, Event, EventAction, ExistingColorHandling, Gradient, Thunderstorm,
    VisualParams, add_color_pair_gradient_frames, adjust_color_pair_brightness, hex_color,
    initialize_raindrop, initialize_spark,
};

impl Effect for Thunderstorm {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // __init__ preamble (effect_thunderstorm.py:188-204): pools are
        // preallocated before build() runs, and the storm clock is read once.
        self.rain_pool
            .preallocate(ctx, 50, initialize_raindrop)
            .map_err(EngineError::Other)?;
        self.spark_gradient = Gradient::with_steps(
            &[
                self.config.spark_glow_color,
                ctx.terminal.config.terminal_background_color,
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
        let dynamic_neutral_gray = hex_color("808080");
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
                    ch.animation.input_fg_color,
                    ch.animation.input_bg_color,
                )
            };
            let (visible_colors, restore_colors) = if dynamic {
                let visible =
                    ColorPair::new(Some(input_fg.unwrap_or(dynamic_neutral_gray)), input_bg);
                let restore = ColorPair::new(input_fg, input_bg);
                (visible, restore)
            } else {
                let visible = ColorPair::new(
                    Some(
                        *final_gradient_mapping
                            .get(&input_coord)
                            .ok_or_else(|| EngineError::Other(String::new()))?,
                    ),
                    None,
                );
                (visible, visible)
            };
            let storm_colors = adjust_color_pair_brightness(&visible_colors, 0.5);
            // upstream stores visible/storm/restore in three maps that are
            // never read back; not stored here.

            // post-strike glow and cool scene
            let glow_fg_gradient = Gradient::with_steps(
                &[
                    self.config.glowing_text_color,
                    storm_colors
                        .fg_color
                        .ok_or_else(|| EngineError::Other(String::new()))?,
                ],
                7,
                false,
            )
            .map_err(EngineError::Other)?;
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let glow_scn = ch.animation.new_scene(false, None, None, "glow", uses_pre);
                let scene = ch
                    .animation
                    .scenes
                    .get_mut(&glow_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                for color in &glow_fg_gradient.spectrum {
                    scene
                        .add_frame(
                            &input_symbol,
                            self.config.text_glow_time,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(*color), storm_colors.bg_color)),
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
                                colors: Some(storm_colors),
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
                    let scene = ch
                        .animation
                        .scenes
                        .get_mut(&fade_scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?;
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
                                colors: Some(storm_colors),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    let fade_gradient = Gradient::with_steps(
                        &[
                            visible_colors
                                .fg_color
                                .ok_or_else(|| EngineError::Other(String::new()))?,
                            storm_colors
                                .fg_color
                                .ok_or_else(|| EngineError::Other(String::new()))?,
                        ],
                        7,
                        false,
                    )
                    .map_err(EngineError::Other)?;
                    let scene = ch
                        .animation
                        .scenes
                        .get_mut(&fade_scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    for color in &fade_gradient.spectrum {
                        scene
                            .add_frame(
                                &input_symbol,
                                12,
                                VisualParams {
                                    colors: Some(ColorPair::new(Some(*color), None)),
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
