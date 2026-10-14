// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for BouncyBalls {
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
        let canvas_top = ctx.terminal.canvas.top;
        for id in characters {
            let (input_coord, input_symbol, input_fg, input_bg, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.animation.input_fg_color.clone(),
                    ch.animation.input_bg_color.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            self.character_final_color_map.insert(
                id,
                final_gradient_mapping.get(&input_coord).must_val().clone(),
            );
            let color = ctx.rng.choice(&self.config.ball_colors).clone();
            let symbol = ctx.rng.choice(&self.config.ball_symbols).clone();
            let ball_scene = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                ch.animation
                    .scenes
                    .get_mut(&scene_id)
                    .must_val()
                    .add_frame(
                        &symbol,
                        1,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(color.clone()), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                scene_id
            };
            let final_scene = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "", uses_pre)
            };
            if dynamic {
                let fg_gradient = match &input_fg {
                    Some(fg) => Some(
                        Gradient::with_steps(&[color.clone(), fg.clone()], 10, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let bg_gradient = match &input_bg {
                    Some(bg) => Some(
                        Gradient::with_steps(&[color.clone(), bg.clone()], 10, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene = ch.animation.scenes.get_mut(&final_scene).must_val();
                if fg_gradient.is_some() || bg_gradient.is_some() {
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            6,
                            fg_gradient.as_ref(),
                            bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    scene
                        .add_frame(
                            &input_symbol,
                            6,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            } else {
                let final_color = self.character_final_color_map.get(&id).must_val().clone();
                let char_final_gradient =
                    Gradient::with_steps(&[color.clone(), final_color], 10, false)
                        .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&final_scene)
                    .must_val()
                    .apply_gradient_to_symbols(
                        &[input_symbol.clone()],
                        6,
                        Some(&char_final_gradient),
                        None,
                    )
                    .map_err(EngineError::Other)?;
            }
            // Coord(input column, int(canvas.top * uniform(1.0, 1.5))) — int() truncation
            let drop_row = (canvas_top as f64 * ctx.rng.uniform(1.0, 1.5)) as i64;
            let input_coord_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .set_coordinate(Coord::new(input_coord.column, drop_row));
                let path_id = ch
                    .motion
                    .new_path(
                        self.config.movement_speed,
                        Some(self.config.movement_easing),
                        None,
                        0,
                        false,
                        "",
                    )
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .must_val()
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
                path_id
            };
            ctx.activate_path(self, id, &input_coord_path);
            ctx.activate_scene(self, id, &ball_scene);
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(input_coord_path),
                EventAction::ActivateScene(final_scene),
            )
            .map_err(EngineError::Other)?;
            self.pending_chars.push(id);
        }
        let mut sorted_chars = self.pending_chars.clone();
        sorted_chars.sort_by_key(|&id| ctx.terminal.arena[id.0 as usize].input_coord.row);
        for id in sorted_chars {
            let row = ctx.terminal.arena[id.0 as usize].input_coord.row;
            self.group_by_row.entry(row).or_default().push(id);
        }
        self.pending_chars.clear();
        self.ball_delay = 0;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.group_by_row.is_empty()
            || !ctx.active_characters.is_empty()
            || !self.pending_chars.is_empty()
        {
            if self.pending_chars.is_empty() && !self.group_by_row.is_empty() {
                let min_row = *self.group_by_row.keys().next().must_val();
                let group = self.group_by_row.remove(&min_row).must_val();
                self.pending_chars.extend(group);
            }
            if !self.pending_chars.is_empty() {
                if self.ball_delay == 0 {
                    for _ in 0..ctx.rng.randint(2, 6) {
                        if self.pending_chars.is_empty() {
                            break;
                        }
                        let index =
                            ctx.rng.randint(0, self.pending_chars.len() as i64 - 1) as usize;
                        let next_character = self.pending_chars.remove(index);
                        ctx.terminal.set_character_visibility(next_character, true);
                        ctx.active_characters.insert(next_character);
                    }
                    self.ball_delay = self.config.ball_delay;
                } else {
                    self.ball_delay -= 1;
                }
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}
impl Default for BouncyBallsConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for BouncyBalls {
    fn default() -> Self {
        Self::new(BouncyBallsConfig::default())
    }
}
