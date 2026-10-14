// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl BinaryPath {
    pub(super) fn setup_binarypath_scenes(
        &mut self,
        ctx: &mut EngineCtx,
        characters: &[CharId],
    ) -> Result<(), EngineError> {
        for &id in characters {
            let (input_symbol, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (ch.input_symbol.clone(), ch.uses_input_preexisting_colors)
            };
            let final_colors = self.character_final_color_map.get(&id).must_val().clone();
            let final_fg_color = final_colors.fg_color.clone();
            let final_bg_color = final_colors.bg_color.clone();
            let dim_fg_color = final_fg_color
                .as_ref()
                .map(|c| Animation::adjust_color_brightness(c, 0.5));
            let dim_bg_color = final_bg_color
                .as_ref()
                .map(|c| Animation::adjust_color_brightness(c, 0.5));
            let collapse_fg_gradient = match &dim_fg_color {
                Some(dim) => Some(
                    Gradient::with_steps(
                        &[Color::from_hex("ffffff").must_val(), dim.clone()],
                        7,
                        false,
                    )
                    .map_err(EngineError::Other)?,
                ),
                None => None,
            };
            let collapse_bg_gradient = match &dim_bg_color {
                Some(dim) => Some(
                    Gradient::with_steps(
                        &[Color::from_hex("ffffff").must_val(), dim.clone()],
                        7,
                        false,
                    )
                    .map_err(EngineError::Other)?,
                ),
                None => None,
            };
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let collapse_scn = ch.animation.new_scene(
                    false,
                    None,
                    Some(Easing::InQuad),
                    "collapse_scn",
                    uses_pre,
                );
                let scene = ch.animation.scenes.get_mut(&collapse_scn).must_val();
                if collapse_fg_gradient.is_some() || collapse_bg_gradient.is_some() {
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            3,
                            collapse_fg_gradient.as_ref(),
                            collapse_bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
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

            let brighten_fg_gradient = match (&dim_fg_color, &final_fg_color) {
                (Some(dim), Some(final_fg)) => Some(
                    Gradient::with_steps(&[dim.clone(), final_fg.clone()], 10, false)
                        .map_err(EngineError::Other)?,
                ),
                _ => None,
            };
            let brighten_bg_gradient = match (&dim_bg_color, &final_bg_color) {
                (Some(dim), Some(final_bg)) => Some(
                    Gradient::with_steps(&[dim.clone(), final_bg.clone()], 10, false)
                        .map_err(EngineError::Other)?,
                ),
                _ => None,
            };
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let brighten_scn =
                    ch.animation
                        .new_scene(false, None, None, "brighten_scn", uses_pre);
                let scene = ch.animation.scenes.get_mut(&brighten_scn).must_val();
                if brighten_fg_gradient.is_some() || brighten_bg_gradient.is_some() {
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            2,
                            brighten_fg_gradient.as_ref(),
                            brighten_bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    scene
                        .add_frame(
                            &input_symbol,
                            2,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn step_scene(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.complete || !ctx.active_characters.is_empty() {
            if self.phase == Phase::Travel {
                while (self.active_binary_reps.len() as i64) < self.max_active_binary_groups
                    && !self.pending_binary_representations.is_empty()
                {
                    let index = ctx
                        .rng
                        .randrange(0, self.pending_binary_representations.len() as i64)
                        as usize;
                    let mut next_binary_rep = self.pending_binary_representations.remove(index);
                    next_binary_rep.is_active = true;
                    self.active_binary_reps.push(next_binary_rep);
                }

                if !self.active_binary_reps.is_empty() {
                    let mut active_reps = std::mem::take(&mut self.active_binary_reps);
                    for active_rep in &mut active_reps {
                        if !active_rep.pending_binary_characters.is_empty() {
                            let next_char = active_rep.pending_binary_characters.remove(0);
                            ctx.active_characters.insert(next_char);
                            ctx.terminal.set_character_visibility(next_char, true);
                        } else if active_rep.travel_complete(ctx) {
                            // _deactivate
                            for &bin_char in &active_rep.binary_characters {
                                ctx.terminal.set_character_visibility(bin_char, false);
                            }
                            active_rep.is_active = false;
                            // _activate_source_character
                            ctx.terminal
                                .set_character_visibility(active_rep.character, true);
                            ctx.activate_scene(self, active_rep.character, "collapse_scn");
                            ctx.active_characters.insert(active_rep.character);
                        }
                    }
                    active_reps.retain(|binary_rep| binary_rep.is_active);
                    self.active_binary_reps = active_reps;
                }

                if ctx.active_characters.is_empty() {
                    self.phase = Phase::Wipe;
                }
            }

            if self.phase == Phase::Wipe {
                for _ in 0..2 {
                    if !self.final_wipe_chars.is_empty() {
                        let next_group = self.final_wipe_chars.remove(0);
                        for character in next_group {
                            ctx.activate_scene(self, character, "brighten_scn");
                            ctx.terminal.set_character_visibility(character, true);
                            ctx.active_characters.insert(character);
                        }
                    } else {
                        self.complete = true;
                    }
                }
            }

            ctx.update(self);
            return Some(emit_frame(ctx));
        }

        if !self.last_frame_provided {
            self.last_frame_provided = true;
            return Some(emit_frame(ctx));
        }

        None
    }
}
impl Default for BinaryPathConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for BinaryPath {
    fn default() -> Self {
        Self::new(BinaryPathConfig::default())
    }
}
