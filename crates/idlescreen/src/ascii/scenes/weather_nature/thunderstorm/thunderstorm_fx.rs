// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Thunderstorm {
    pub(super) fn setup_thunderstorm_unfade_flash(
        &mut self,
        ctx: &mut EngineCtx,
        id: CharId,
        input_symbol: &str,
        uses_pre: bool,
        dynamic: bool,
        visible_colors: &ColorPair,
        restore_colors: &ColorPair,
        storm_colors: &ColorPair,
    ) -> Result<(), EngineError> {
        let input_symbol = input_symbol.to_string();
        let visible_colors = visible_colors.clone();
        let restore_colors = restore_colors.clone();
        let storm_colors = storm_colors.clone();
        {
            // unfade scene
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let unfade_scn = ch
                    .animation
                    .new_scene(false, None, None, "unfade", uses_pre);
                if dynamic {
                    let scene = ch.animation.scenes.get_mut(&unfade_scn).must_val();
                    add_color_pair_gradient_frames(
                        scene,
                        &input_symbol,
                        &storm_colors,
                        &visible_colors,
                        7,
                        12,
                    )
                    .map_err(EngineError::Other)?;
                    scene
                        .add_frame(
                            &input_symbol,
                            12,
                            VisualParams {
                                colors: Some(visible_colors.clone()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                    if restore_colors != visible_colors {
                        scene
                            .add_frame(
                                &input_symbol,
                                12,
                                VisualParams {
                                    colors: Some(restore_colors.clone()),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                } else {
                    let unfade_gradient: Vec<Color> = Gradient::with_steps(
                        &[
                            visible_colors.fg_color.clone().must_val(),
                            storm_colors.fg_color.clone().must_val(),
                        ],
                        7,
                        false,
                    )
                    .map_err(EngineError::Other)?
                    .spectrum
                    .into_iter()
                    .rev()
                    .collect();
                    let scene = ch.animation.scenes.get_mut(&unfade_scn).must_val();
                    for color in &unfade_gradient {
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

            // lightning flash scene
            let lightning_flash_color = Animation::adjust_color_brightness(
                visible_colors.fg_color.as_ref().must_val(),
                1.7,
            );
            let flash_gradient = Gradient::with_steps(
                &[
                    storm_colors.fg_color.clone().must_val(),
                    lightning_flash_color,
                ],
                7,
                true,
            )
            .map_err(EngineError::Other)?;
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let strike_scn = ch.animation.new_scene(false, None, None, "flash", uses_pre);
                let scene = ch.animation.scenes.get_mut(&strike_scn).must_val();
                for color in &flash_gradient.spectrum {
                    scene
                        .add_frame(
                            &input_symbol,
                            6,
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
            }

            ctx.terminal.set_character_visibility(id, true);
        }
        Ok(())
    }

    pub(super) fn step_scene(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if ctx.active_characters.is_empty() && self.phase == Phase::Complete {
            return None;
        }
        match self.phase {
            Phase::PreStorm => {
                self.pre_storm_text_fade(ctx);
                self.phase = Phase::Waiting;
            }
            Phase::Storm => {
                self.rain(ctx);
                if !self.strike_in_progress && ctx.rng.random() < 0.008 {
                    self.strike_in_progress = true;
                    self.lightning_strike(ctx);
                }
                if self.strike_in_progress {
                    self.step_lightning_strike(ctx);
                }

                for &glow_char in &self.pending_glow_chars {
                    ctx.active_characters.insert(glow_char);
                }
                self.pending_glow_chars.clear();
                if ctx.clock.now_monotonic() - self.storm_start_time
                    >= self.config.storm_time as f64
                    && !self.strike_in_progress
                {
                    self.post_storm_text_fade_in(ctx);
                    self.phase = Phase::Complete;
                }
            }
            Phase::Waiting | Phase::Complete => {}
        }
        ctx.update(self);
        Some(emit_frame(ctx))
    }
}
impl Default for ThunderstormConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Thunderstorm {
    fn default() -> Self {
        Self::new(ThunderstormConfig::default())
    }
}
