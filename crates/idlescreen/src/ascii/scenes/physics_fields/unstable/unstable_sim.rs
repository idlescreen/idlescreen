// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Unstable {
    pub(super) fn setup_unstable_final(
        &mut self,
        ctx: &mut EngineCtx,
        id: CharId,
        input_symbol: &str,
        uses_pre: bool,
        dynamic: bool,
    ) -> Result<(), EngineError> {
        let input_symbol = input_symbol.to_string();
        {
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "final", uses_pre);
            }
            if dynamic {
                let final_pair = self.character_final_color_map.get(&id).must_val().clone();
                let final_fg_color = final_pair.fg_color.clone();
                let final_bg_color = final_pair.bg_color.clone();
                if final_fg_color.is_none() && final_bg_color.is_none() {
                    let fg_gradient = Gradient::with_steps(
                        &[self.config.unstable_color.clone(), neutral_gray()],
                        12,
                        false,
                    )
                    .map_err(EngineError::Other)?;
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let scene = ch.animation.scenes.get_mut("final").must_val();
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            3,
                            Some(&fg_gradient),
                            None,
                        )
                        .map_err(EngineError::Other)?;
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
                } else {
                    let step_grad = |opt: &Option<Color>| {
                        opt.as_ref()
                            .map(|c| {
                                Gradient::with_steps(
                                    &[self.config.unstable_color.clone(), c.clone()],
                                    12,
                                    false,
                                )
                                .map_err(EngineError::Other)
                            })
                            .transpose()
                    };
                    let fg_gradient = step_grad(&final_fg_color)?;
                    let bg_gradient = step_grad(&final_bg_color)?;
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let scene = ch.animation.scenes.get_mut("final").must_val();
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            3,
                            fg_gradient.as_ref(),
                            bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                    if final_fg_color.is_none() {
                        scene
                            .add_frame(
                                &input_symbol,
                                3,
                                VisualParams {
                                    colors: Some(ColorPair::new(None, final_bg_color)),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                }
            } else {
                let final_fg_color = self
                    .character_final_color_map
                    .get(&id)
                    .must_val()
                    .fg_color
                    .clone()
                    .unwrap_or_else(neutral_gray);
                let final_color = Gradient::with_steps(
                    &[self.config.unstable_color.clone(), final_fg_color],
                    12,
                    false,
                )
                .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("final")
                    .must_val()
                    .apply_gradient_to_symbols(&[input_symbol.clone()], 3, Some(&final_color), None)
                    .map_err(EngineError::Other)?;
            }
            ctx.activate_scene(self, id, "rumble");
            if dynamic {
                let start_pair = self.character_start_color_map.get(&id).must_val().clone();
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let input_symbol = ch.input_symbol.clone();
                let uses = ch.uses_input_preexisting_colors;
                ch.animation.set_appearance(
                    &input_symbol,
                    uses,
                    Some(&input_symbol.clone()),
                    Some(start_pair),
                );
            }
            ctx.terminal.set_character_visibility(id, true);
        }
        Ok(())
    }
}
impl Unstable {
    pub(super) fn step_scene(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        let mut next_frame: Option<String> = None;
        if self.phase == Phase::Rumble {
            if self.current_rumble_steps < self.max_rumble_steps {
                if self.current_rumble_steps > 30
                    && self.current_rumble_steps % self.rumble_mod_delay == 0
                {
                    let row_offset = *ctx.rng.choice(&[-1i64, 0, 1]);
                    let column_offset = *ctx.rng.choice(&[-1i64, 0, 1]);
                    let characters = ctx.terminal.get_characters(
                        &mut ctx.rng,
                        CharacterFilter::default(),
                        CharacterSort::TopToBottomLeftToRight,
                    );
                    for &id in &characters {
                        {
                            let motion = &mut ctx.terminal.arena[id.0 as usize].motion;
                            let current = motion.current_coord;
                            motion.set_coordinate(Coord::new(
                                current.column + column_offset,
                                current.row + row_offset,
                            ));
                        }
                        ctx.step_animation(self, id);
                    }
                    next_frame = Some(emit_frame(ctx));
                    for &id in &characters {
                        let jumbled = *self.jumbled_coords.get(&id).must_val();
                        ctx.terminal.arena[id.0 as usize]
                            .motion
                            .set_coordinate(jumbled);
                    }
                    self.rumble_mod_delay = (self.rumble_mod_delay - 1).max(1);
                } else {
                    let characters = ctx.terminal.get_characters(
                        &mut ctx.rng,
                        CharacterFilter::default(),
                        CharacterSort::TopToBottomLeftToRight,
                    );
                    for &id in &characters {
                        ctx.step_animation(self, id);
                    }
                    next_frame = Some(emit_frame(ctx));
                }
                self.current_rumble_steps += 1;
            } else {
                self.phase = Phase::Explosion;
                let characters = ctx.terminal.get_characters(
                    &mut ctx.rng,
                    CharacterFilter::default(),
                    CharacterSort::TopToBottomLeftToRight,
                );
                for &id in &characters {
                    ctx.activate_path(self, id, "explosion");
                }
                ctx.active_characters.clear();
                ctx.active_characters.extend(characters);
            }
        }

        if self.phase == Phase::Explosion {
            if !ctx.active_characters.is_empty() {
                let snapshot: Vec<CharId> = ctx.active_characters.iter().collect();
                for id in snapshot {
                    ctx.tick(self, id);
                }
                let retained: Vec<CharId> = ctx
                    .active_characters
                    .iter()
                    .filter(|&id| {
                        let ch = &ctx.terminal.arena[id.0 as usize];
                        let explosion_target =
                            ch.motion.paths.get("explosion").must_val().waypoints[0].coord;
                        ch.motion.current_coord != explosion_target
                    })
                    .collect();
                ctx.active_characters.clear();
                ctx.active_characters.extend(retained);
                next_frame = Some(emit_frame(ctx));
            } else if self.explosion_hold_time != 0 {
                self.explosion_hold_time -= 1;
                next_frame = Some(emit_frame(ctx));
            } else {
                self.phase = Phase::Reassembly;
                let characters = ctx.terminal.get_characters(
                    &mut ctx.rng,
                    CharacterFilter::default(),
                    CharacterSort::TopToBottomLeftToRight,
                );
                for &id in &characters {
                    ctx.activate_scene(self, id, "final");
                    ctx.active_characters.insert(id);
                    ctx.activate_path(self, id, "reassembly");
                }
            }
        }

        if self.phase == Phase::Reassembly && !ctx.active_characters.is_empty() {
            // Upstream iterates the active_characters set (effect_unstable.py:354);
            // canonical order is ascending character_id (shim patched to match).
            let snapshot: Vec<CharId> = ctx.active_characters.iter().collect();
            for id in snapshot {
                ctx.tick(self, id);
            }
            let retained: Vec<CharId> = ctx
                .active_characters
                .iter()
                .filter(|&id| {
                    let ch = &ctx.terminal.arena[id.0 as usize];
                    let reassembly_target =
                        ch.motion.paths.get("reassembly").must_val().waypoints[0].coord;
                    ch.motion.current_coord != reassembly_target
                        || !ch.animation.active_scene_is_complete()
                })
                .collect();
            ctx.active_characters.clear();
            ctx.active_characters.extend(retained);
            next_frame = Some(emit_frame(ctx));
        }

        next_frame
    }
}
pub(super) fn uses_pre_of(ch: &ttfx::engine::character::EffectCharacter) -> bool {
    ch.uses_input_preexisting_colors
}
