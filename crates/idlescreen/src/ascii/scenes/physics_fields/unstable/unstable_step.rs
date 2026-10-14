// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for Unstable {
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
            let ch = &ctx.terminal.arena[id.0 as usize];
            let (start_colors, final_colors) = if dynamic {
                let start_fg_color = ch
                    .animation
                    .input_fg_color
                    .clone()
                    .unwrap_or_else(neutral_gray);
                (
                    ColorPair::new(Some(start_fg_color), ch.animation.input_bg_color.clone()),
                    ColorPair::new(
                        ch.animation.input_fg_color.clone(),
                        ch.animation.input_bg_color.clone(),
                    ),
                )
            } else {
                let start = ColorPair::new(
                    Some(
                        final_gradient_mapping
                            .get(&ch.input_coord)
                            .must_val()
                            .clone(),
                    ),
                    None,
                );
                (start.clone(), start)
            };
            self.character_start_color_map.insert(id, start_colors);
            self.character_final_color_map.insert(id, final_colors);
        }
        let mut character_coords: Vec<Coord> = characters
            .iter()
            .map(|&id| ctx.terminal.arena[id.0 as usize].input_coord)
            .collect();
        for &id in &characters {
            let pos = ctx.rng.randint(0, 3);
            let (col, row) = match pos {
                0 => {
                    let col = ctx.terminal.canvas.left;
                    let row = ctx.terminal.canvas.random_row(&mut ctx.rng, false);
                    (col, row)
                }
                1 => {
                    let col = ctx.terminal.canvas.right;
                    let row = ctx.terminal.canvas.random_row(&mut ctx.rng, false);
                    (col, row)
                }
                2 => {
                    let col = ctx.terminal.canvas.random_column(&mut ctx.rng, false);
                    let row = ctx.terminal.canvas.bottom;
                    (col, row)
                }
                _ => {
                    let col = ctx.terminal.canvas.random_column(&mut ctx.rng, false);
                    let row = ctx.terminal.canvas.top;
                    (col, row)
                }
            };
            let jumbled_coord = character_coords
                .remove(ctx.rng.randint(0, character_coords.len() as i64 - 1) as usize);
            self.jumbled_coords.insert(id, jumbled_coord);
            let (input_symbol, input_coord, uses_pre) = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion.set_coordinate(jumbled_coord);
                let explosion_path = ch
                    .motion
                    .new_path(
                        self.config.explosion_speed,
                        Some(self.config.explosion_ease),
                        None,
                        0,
                        false,
                        "explosion",
                    )
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&explosion_path)
                    .must_val()
                    .new_waypoint(Coord::new(col, row), None, "")
                    .map_err(EngineError::Other)?;
                let reassembly_path = ch
                    .motion
                    .new_path(
                        self.config.reassembly_speed,
                        Some(self.config.reassembly_ease),
                        None,
                        0,
                        false,
                        "reassembly",
                    )
                    .map_err(EngineError::Other)?;
                let input_coord = ch.input_coord;
                ch.motion
                    .paths
                    .get_mut(&reassembly_path)
                    .must_val()
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
                ch.animation
                    .new_scene(false, None, None, "rumble", uses_pre_of(ch));
                (ch.input_symbol.clone(), input_coord, uses_pre_of(ch))
            };
            let _ = input_coord;
            if dynamic {
                let start_pair = self.character_start_color_map.get(&id).must_val().clone();
                let start_fg_color = start_pair.fg_color.clone().unwrap_or_else(neutral_gray);
                let start_bg_color = start_pair.bg_color.clone();
                let fg_gradient = Gradient::with_steps(
                    &[start_fg_color, self.config.unstable_color.clone()],
                    12,
                    false,
                )
                .map_err(EngineError::Other)?;
                let bg_gradient = match start_bg_color {
                    Some(bg) => Some(
                        Gradient::with_steps(&[bg, self.config.unstable_color.clone()], 12, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("rumble")
                    .must_val()
                    .apply_gradient_to_symbols(
                        &[input_symbol.clone()],
                        10,
                        Some(&fg_gradient),
                        bg_gradient.as_ref(),
                    )
                    .map_err(EngineError::Other)?;
            } else {
                let final_fg_color = self
                    .character_final_color_map
                    .get(&id)
                    .must_val()
                    .fg_color
                    .clone()
                    .unwrap_or_else(neutral_gray);
                let unstable_gradient = Gradient::with_steps(
                    &[final_fg_color, self.config.unstable_color.clone()],
                    12,
                    false,
                )
                .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("rumble")
                    .must_val()
                    .apply_gradient_to_symbols(
                        &[input_symbol.clone()],
                        10,
                        Some(&unstable_gradient),
                        None,
                    )
                    .map_err(EngineError::Other)?;
            }
            self.setup_unstable_final(ctx, id, &input_symbol, uses_pre, dynamic)?;
        }
        self.explosion_hold_time = 30;
        self.phase = Phase::Rumble;
        self.max_rumble_steps = 150;
        self.current_rumble_steps = 0;
        self.rumble_mod_delay = 18;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        self.step_scene(ctx)
    }
}
