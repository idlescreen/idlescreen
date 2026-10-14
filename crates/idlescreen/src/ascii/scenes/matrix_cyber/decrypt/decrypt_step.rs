// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Decrypt {
    /// DecryptIterator.prepare_data_for_type_effect.
    pub(super) fn prepare_data_for_type_effect(
        &mut self,
        ctx: &mut EngineCtx,
    ) -> Result<(), EngineError> {
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            let uses_pre = ctx.terminal.arena[id.0 as usize].uses_input_preexisting_colors;
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .new_scene(false, None, None, "typing", uses_pre);
            }
            for block_char in ["▉", "▓", "▒", "░"] {
                let color = ctx.rng.choice(&self.config.ciphertext_colors).clone();
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("typing")
                    .must_val()
                    .add_frame(
                        block_char,
                        2,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(color), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            let symbol = ctx.rng.choice(&self.encrypted_symbols).clone();
            let color = ctx.rng.choice(&self.config.ciphertext_colors).clone();
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("typing")
                    .must_val()
                    .add_frame(
                        &symbol,
                        1,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(color), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            self.typing_pending_chars.push(id);
        }
        Ok(())
    }

    /// DecryptIterator.prepare_data_for_decrypt_effect.
    pub(super) fn prepare_data_for_decrypt_effect(
        &mut self,
        ctx: &mut EngineCtx,
    ) -> Result<(), EngineError> {
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            self.make_decrypting_animation_scenes(ctx, id)?;
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene("fast_decrypt".to_string()),
                EventAction::ActivateScene("slow_decrypt".to_string()),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene("slow_decrypt".to_string()),
                EventAction::ActivateScene("discovered".to_string()),
            )
            .map_err(EngineError::Other)?;
            ctx.activate_scene(self, id, "fast_decrypt");
            self.decrypting_pending_chars.push(id);
        }
        Ok(())
    }
}
impl EffectHooks for Decrypt {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
impl Effect for Decrypt {
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
                    ColorPair::new(
                        ch.animation.input_fg_color.clone(),
                        ch.animation.input_bg_color.clone(),
                    )
                } else {
                    ColorPair::new(
                        Some(
                            final_gradient_mapping
                                .get(&ch.input_coord)
                                .must_val()
                                .clone(),
                        ),
                        None,
                    )
                }
            };
            self.character_final_color_map.insert(id, final_colors);
        }
        self.prepare_data_for_type_effect(ctx)?;
        self.prepare_data_for_decrypt_effect(ctx)?;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.phase == Phase::Typing {
            if !self.typing_pending_chars.is_empty() || !ctx.active_characters.is_empty() {
                if !self.typing_pending_chars.is_empty() && ctx.rng.randint(0, 100) <= 75 {
                    for _ in 0..self.config.typing_speed {
                        if !self.typing_pending_chars.is_empty() {
                            let next_character = self.typing_pending_chars.remove(0);
                            ctx.terminal.set_character_visibility(next_character, true);
                            ctx.activate_scene(self, next_character, "typing");
                            ctx.active_characters.insert(next_character);
                        }
                    }
                }
                ctx.update(self);
                return Some(emit_frame(ctx));
            }
            // Upstream iterates the decrypting_pending_chars set here to
            // activate "fast_decrypt" (effect_decrypt.py:263) — activation
            // only mutates each character itself (no handlers registered for
            // SCENE_ACTIVATED), so order is unobservable; ttfx iterates the
            // canonical ascending-id order.
            ctx.active_characters.clear();
            ctx.active_characters
                .extend(self.decrypting_pending_chars.iter().copied());
            let active: Vec<CharId> = ctx.active_characters.iter().collect();
            for id in active {
                ctx.activate_scene(self, id, "fast_decrypt");
            }
            self.phase = Phase::Decrypting;
        }

        if self.phase == Phase::Decrypting {
            if !ctx.active_characters.is_empty() {
                ctx.update(self);
                return Some(emit_frame(ctx));
            }
            return None;
        }
        None
    }
}
impl Default for DecryptConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Decrypt {
    fn default() -> Self {
        Self::new(DecryptConfig::default())
    }
}
