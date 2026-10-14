// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Thunderstorm {
    /// ThunderstormIterator.lightning_strike.
    pub(super) fn lightning_strike(&mut self, ctx: &mut EngineCtx) {
        self.setup_lightning_strike(ctx, None);
        let strike_base_color = self.config.lightning_color.clone();
        let strike_flash_color = Animation::adjust_color_brightness(&strike_base_color, 1.7);
        let strike_gradient =
            Gradient::with_steps(&[strike_base_color.clone(), strike_flash_color], 7, true)
                .must_val();
        let fade_gradient = Gradient::with_steps(
            &[
                strike_base_color,
                ctx.terminal.config.terminal_background_color.clone(),
            ],
            6,
            false,
        )
        .must_val();
        let layer = 1;
        let flash_ease = Easing::CubicBezier(0.0, 1.6, 1.0, ctx.rng.uniform(-0.6, 0.4));
        for &strike_char in &self.pending_strike_chars {
            let symbol = ctx.terminal.arena[strike_char.0 as usize]
                .animation
                .current_character_visual
                .symbol
                .clone();
            {
                let ch = &mut ctx.terminal.arena[strike_char.0 as usize];
                let uses_pre = ch.uses_input_preexisting_colors;
                let flash_scn =
                    ch.animation
                        .new_scene(false, None, Some(flash_ease), "flash", uses_pre);
                let scene = ch.animation.scenes.get_mut(&flash_scn).must_val();
                for color in &strike_gradient.spectrum {
                    scene
                        .add_frame(
                            &symbol,
                            6,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(color.clone()), None)),
                                ..Default::default()
                            },
                        )
                        .must_val();
                }
                let fade_scn = ch.animation.new_scene(false, None, None, "fade", uses_pre);
                let scene = ch.animation.scenes.get_mut(&fade_scn).must_val();
                for color in &fade_gradient.spectrum {
                    scene
                        .add_frame(
                            &symbol,
                            2,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(color.clone()), None)),
                                ..Default::default()
                            },
                        )
                        .must_val();
                }
                ch.layer = layer;
            }
            ctx.register_event(
                strike_char,
                Event::SceneComplete,
                CallerKey::Scene("flash".to_string()),
                EventAction::ActivateScene("fade".to_string()),
            )
            .must_val();
            ctx.register_event(
                strike_char,
                Event::SceneComplete,
                CallerKey::Scene("fade".to_string()),
                EventAction::Callback(EffectCallback {
                    id: CB_HIDE_CHARACTER,
                    args: Vec::new(),
                }),
            )
            .must_val();
            ctx.register_event(
                strike_char,
                Event::SceneComplete,
                CallerKey::Scene("fade".to_string()),
                EventAction::Callback(EffectCallback {
                    id: CB_MAKE_CHAR_GLOW,
                    args: Vec::new(),
                }),
            )
            .must_val();
            ctx.register_event(
                strike_char,
                Event::SceneComplete,
                CallerKey::Scene("fade".to_string()),
                EventAction::Callback(EffectCallback {
                    id: CB_RETURN_STRIKE_TO_POOL,
                    args: Vec::new(),
                }),
            )
            .must_val();
        }

        let text_chars = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in text_chars {
            ctx.terminal.arena[id.0 as usize]
                .animation
                .scenes
                .get_mut("flash")
                .must_val()
                .ease = Some(flash_ease);
        }
    }

    /// ThunderstormIterator.step_lightning_strike.
    pub(super) fn step_lightning_strike(&mut self, ctx: &mut EngineCtx) {
        if self.strike_progression_delay != 0 {
            self.strike_progression_delay -= 1;
            return;
        }
        if !self.pending_strike_chars.is_empty() {
            let batch = ctx.rng.randint(1, 3);
            for _ in 0..batch {
                if self.pending_strike_chars.is_empty() {
                    break;
                }
                let next_strike_char = self.pending_strike_chars.remove(0);
                self.active_strike_chars.push(next_strike_char);
                ctx.terminal
                    .set_character_visibility(next_strike_char, true);
                self.strike_progression_delay = 1;

                // if the last strike_char was activated, activate the sparks
                // and setup the post-fade callback to indicate the strike has
                // ended
                if self.pending_strike_chars.is_empty() {
                    let spark_gradient = self.spark_gradient.clone();
                    let spark_glow_time = self.config.spark_glow_time;
                    let spark_count = ctx.rng.randint(12, 18);
                    for _ in 0..spark_count {
                        let origin = ctx.terminal.arena
                            [self.active_strike_chars.last().must_val().0 as usize]
                            .motion
                            .current_coord;
                        let _ = self.spark_pool.emit(
                            ctx,
                            origin,
                            None,
                            true,
                            ParticleReset {
                                clear_events: true,
                                ..Default::default()
                            },
                            |ctx, particle| {
                                initialize_spark(ctx, particle, &spark_gradient, spark_glow_time)
                            },
                            setup_sparks_for_impact,
                        );
                    }
                    ctx.register_event(
                        next_strike_char,
                        Event::SceneComplete,
                        CallerKey::Scene("fade".to_string()),
                        EventAction::Callback(EffectCallback {
                            id: CB_SET_STRIKE_IN_PROGRESS_FALSE,
                            args: Vec::new(),
                        }),
                    )
                    .must_val();

                    // activate the flash scene on all strike chars and text
                    let strikes = std::mem::take(&mut self.active_strike_chars);
                    for &strike_char in &strikes {
                        ctx.activate_scene(self, strike_char, "flash");
                        ctx.active_characters.insert(strike_char);
                    }
                    // (take() above already performed active_strike_chars.clear())

                    let text_chars = {
                        let filter = CharacterFilter::default();
                        ctx.terminal.get_characters(
                            &mut ctx.rng,
                            filter,
                            CharacterSort::TopToBottomLeftToRight,
                        )
                    };
                    for id in text_chars {
                        ctx.activate_scene(self, id, "flash");
                        ctx.active_characters.insert(id);
                    }
                }
            }
        }
    }

    /// ThunderstormIterator.rain.
    pub(super) fn rain(&mut self, ctx: &mut EngineCtx) {
        if self.delay != 0 {
            self.delay -= 1;
            return;
        }
        let count = ctx.rng.randint(1, 6);
        for _ in 0..count {
            let spawn_column = ctx
                .rng
                .randint(1 - ctx.terminal.canvas.top, ctx.terminal.canvas.right);
            let origin = Coord::new(spawn_column - 1, ctx.terminal.canvas.top + 1);
            let _ = self.rain_pool.emit(
                ctx,
                origin,
                None,
                true,
                ParticleReset {
                    clear_events: true,
                    ..Default::default()
                },
                initialize_raindrop,
                setup_raindrop,
            );
        }
        self.delay = ctx.rng.randint(1, 7);
    }
}
impl Thunderstorm {
    /// ThunderstormIterator.pre_storm_text_fade.
    pub(super) fn pre_storm_text_fade(&mut self, ctx: &mut EngineCtx) {
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            ctx.activate_scene(self, id, "fade");
            ctx.active_characters.insert(id);
        }
    }

    /// ThunderstormIterator.post_storm_text_fade_in.
    pub(super) fn post_storm_text_fade_in(&mut self, ctx: &mut EngineCtx) {
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            ctx.activate_scene(self, id, "unfade");
            ctx.active_characters.insert(id);
        }
    }
}
