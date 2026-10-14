// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for Burn {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // BurnIterator.__init__ order: PrimsSimple (random starting coord),
        // then the pool preallocation (2000 choice() draws + initializer each).
        let mut algo = PrimsSimple::new(ctx, None, true).map_err(EngineError::Other)?;
        let mut pool = ParticlePool::new(
            SMOKE_SYMBOLS.iter().map(|s| s.to_string()).collect(),
            Some(2000),
            None,
        )
        .map_err(EngineError::Other)?;
        pool.preallocate(ctx, 2000, Self::initialize_smoke)
            .map_err(EngineError::Other)?;
        self.smoke_particles = Some(pool);

        // BurnIterator.build()
        let burn_char_order: Vec<String> = BURN_CHAR_ORDER.iter().map(|s| s.to_string()).collect();
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
            let coord = ctx.terminal.arena[id.0 as usize].input_coord;
            self.character_final_color_map
                .insert(id, final_gradient_mapping.get(&coord).must_val().clone());
        }
        let fire_gradient = Gradient::with_steps(&self.config.burn_colors, 10, false)
            .map_err(EngineError::Other)?;

        while !algo.complete {
            algo.step(ctx);
        }

        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for &id in &characters {
            ctx.terminal.set_character_visibility(id, true);
            let (input_symbol, input_fg, input_bg, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_symbol.clone(),
                    ch.animation.input_fg_color.clone(),
                    ch.animation.input_bg_color.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.set_appearance(
                    &input_symbol,
                    uses_pre,
                    Some(&input_symbol),
                    Some(ColorPair::new(
                        Some(self.config.starting_color.clone()),
                        None,
                    )),
                );
            }
            let burn_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene_id = ch.animation.new_scene(false, None, None, "burn", uses_pre);
                ch.animation
                    .scenes
                    .get_mut(&scene_id)
                    .must_val()
                    .apply_gradient_to_symbols(&burn_char_order, 4, Some(&fire_gradient), None)
                    .map_err(EngineError::Other)?;
                scene_id
            };
            let final_color_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "", uses_pre)
            };
            let fire_last = fire_gradient.spectrum.last().must_val().clone();
            if dynamic {
                let fg_gradient = match &input_fg {
                    Some(fg) => Some(
                        Gradient::with_steps(&[fire_last.clone(), fg.clone()], 8, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let bg_gradient = match &input_bg {
                    Some(bg) => Some(
                        Gradient::with_steps(&[fire_last.clone(), bg.clone()], 8, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene = ch.animation.scenes.get_mut(&final_color_scn).must_val();
                if fg_gradient.is_some() || bg_gradient.is_some() {
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            4,
                            fg_gradient.as_ref(),
                            bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    scene
                        .add_frame(
                            &input_symbol,
                            4,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            } else {
                let final_color = self.character_final_color_map.get(&id).must_val().clone();
                let char_gradient = Gradient::with_steps(&[fire_last, final_color], 8, false)
                    .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene = ch.animation.scenes.get_mut(&final_color_scn).must_val();
                for color in &char_gradient.spectrum {
                    scene
                        .add_frame(
                            &input_symbol,
                            4,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(color.clone()), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene(burn_scn.clone()),
                EventAction::ActivateScene(final_color_scn),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene(burn_scn),
                EventAction::Callback(EffectCallback {
                    id: CB_EMIT_SMOKE,
                    args: Vec::new(),
                }),
            )
            .map_err(EngineError::Other)?;
        }
        self.char_link_order = algo.char_link_order;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.char_link_order.is_empty() || !ctx.active_characters.is_empty() {
            for _ in 0..ctx.rng.randint(5, 9) {
                if !self.char_link_order.is_empty() {
                    let next_char = self.char_link_order.remove(0);
                    if !self.is_burnable(ctx, next_char) {
                        continue;
                    }
                    ctx.activate_scene(self, next_char, "burn");
                    ctx.active_characters.insert(next_char);
                }
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}
impl Default for BurnConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Burn {
    fn default() -> Self {
        Self::new(BurnConfig::default())
    }
}
