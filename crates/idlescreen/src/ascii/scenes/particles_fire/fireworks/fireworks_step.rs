// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CallerKey, CharId, CharacterFilter, CharacterSort, Color, ColorPair, Effect, EffectCallback,
    EffectHooks, EngineCtx, EngineError, Event, EventAction, ExistingColorHandling, Fireworks,
    FireworksConfig, Gradient, SyncMetric, VisualParams, emit_frame, hex_color, round_half_even,
};

impl Fireworks {
    /// FireworksIterator.prepare_scenes.
    pub(super) fn prepare_scenes(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
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
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for &id in &characters {
            let final_colors = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                if dynamic {
                    ColorPair::new(ch.animation.input_fg_color, ch.animation.input_bg_color)
                } else {
                    ColorPair::new(
                        Some(
                            *final_gradient_mapping
                                .get(&ch.input_coord)
                                .ok_or_else(|| EngineError::Other(String::new()))?,
                        ),
                        None,
                    )
                }
            };
            self.character_final_color_map.insert(id, final_colors);
        }
        let white = hex_color("FFFFFF");
        let shells = self.shells.clone();
        for firework_shell in shells {
            let shell_color = *ctx.rng.choice(&self.config.firework_colors);
            let shell_gradient = Gradient::with_steps(&[shell_color, white, shell_color], 5, false)
                .map_err(EngineError::Other)?;
            for id in firework_shell {
                let (input_symbol, input_fg, input_bg, uses_pre) = {
                    let ch = &ctx.terminal.arena[id.0 as usize];
                    (
                        ch.input_symbol.clone(),
                        ch.animation.input_fg_color,
                        ch.animation.input_bg_color,
                        ch.uses_input_preexisting_colors,
                    )
                };
                let (launch_scn, bloom_scn, fall_scn) = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let launch_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                    let scene = ch
                        .animation
                        .scenes
                        .get_mut(&launch_id)
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    for (dur, col) in [(2, &shell_color), (1, &white)] {
                        scene
                            .add_frame(
                                &self.config.firework_symbol,
                                dur,
                                VisualParams {
                                    colors: Some(ColorPair::new(Some(*col), None)),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                    scene.is_looping = true;
                    let bloom_id =
                        ch.animation
                            .new_scene(false, Some(SyncMetric::Step), None, "", uses_pre);
                    let scene = ch
                        .animation
                        .scenes
                        .get_mut(&bloom_id)
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    for color in &shell_gradient.spectrum {
                        scene
                            .add_frame(
                                &input_symbol,
                                2,
                                VisualParams {
                                    colors: Some(ColorPair::new(Some(*color), None)),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                    let fall_id = ch
                        .animation
                        .new_scene(false, None, None, "fall_scn", uses_pre);
                    (launch_id, bloom_id, fall_id)
                };
                if dynamic {
                    let step_grad = |opt: &Option<Color>| {
                        opt.as_ref()
                            .map(|c| {
                                Gradient::with_steps(&[shell_color, *c], 15, false)
                                    .map_err(EngineError::Other)
                            })
                            .transpose()
                    };
                    let fg_gradient = step_grad(&input_fg)?;
                    let bg_gradient = step_grad(&input_bg)?;
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let scene = ch
                        .animation
                        .scenes
                        .get_mut(&fall_scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    if fg_gradient.is_some() || bg_gradient.is_some() {
                        scene
                            .apply_gradient_to_symbols(
                                std::slice::from_ref(&input_symbol),
                                10,
                                fg_gradient.as_ref(),
                                bg_gradient.as_ref(),
                            )
                            .map_err(EngineError::Other)?;
                    } else {
                        scene
                            .add_frame(
                                &input_symbol,
                                10,
                                VisualParams {
                                    colors: Some(ColorPair::default()),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                } else {
                    let final_fg = self.character_final_color_map[&id]
                        .fg_color
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    let fall_gradient = Gradient::with_steps(&[shell_color, final_fg], 15, false)
                        .map_err(EngineError::Other)?;
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation
                        .scenes
                        .get_mut(&fall_scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?
                        .apply_gradient_to_symbols(
                            std::slice::from_ref(&input_symbol),
                            10,
                            Some(&fall_gradient),
                            None,
                        )
                        .map_err(EngineError::Other)?;
                }
                ctx.activate_scene(self, id, &launch_scn);
                ctx.register_event(
                    id,
                    Event::PathComplete,
                    CallerKey::Path("apex_pth".to_string()),
                    EventAction::ActivateScene(bloom_scn),
                )
                .map_err(EngineError::Other)?;
                ctx.register_event(
                    id,
                    Event::PathActivated,
                    CallerKey::Path("input_pth".to_string()),
                    EventAction::ActivateScene(fall_scn),
                )
                .map_err(EngineError::Other)?;
            }
        }
        Ok(())
    }
}
impl EffectHooks for Fireworks {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
impl Effect for Fireworks {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // __init__ precomputations (no RNG)
        self.firework_volume = std::cmp::max(
            1,
            round_half_even(
                self.config.firework_volume * ctx.terminal.input_characters.len() as f64,
            ),
        );
        self.explode_distance =
            round_half_even(ctx.terminal.canvas.right as f64 * self.config.explode_distance)
                .clamp(1, 15);
        self.launch_delay = 0;
        self.prepare_waypoints(ctx)?;
        self.prepare_scenes(ctx)?;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.shells.is_empty() || !ctx.active_characters.is_empty() {
            if !self.shells.is_empty() && self.launch_delay <= 0 {
                let next_group = self.shells.pop()?;
                for id in next_group {
                    ctx.terminal.set_character_visibility(id, true);
                    ctx.active_characters.insert(id);
                }
                // int(launch_delay * uniform(0.5, 1.5)) — truncation
                self.launch_delay =
                    (self.config.launch_delay as f64 * ctx.rng.uniform(0.5, 1.5)) as i64;
            }
            self.launch_delay -= 1;
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Fireworks {
    fn default() -> Self {
        Self::new(FireworksConfig::default())
    }
}
