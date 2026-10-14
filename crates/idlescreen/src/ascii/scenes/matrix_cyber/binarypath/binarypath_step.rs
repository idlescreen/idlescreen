// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for BinaryPath {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // __init__: final_wipe_chars computed before build()
        self.final_wipe_chars = ctx.terminal.get_characters_grouped(
            CharacterFilter::default(),
            CharacterGroup::DiagonalTopRightToBottomLeft,
        );

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
            let final_colors = if dynamic {
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
            };
            self.character_final_color_map.insert(id, final_colors);
        }

        for &id in &characters {
            let (symbol, input_coord) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.animation.current_character_visual.symbol.clone(),
                    ch.input_coord,
                )
            };
            // format(ord(symbol), "08b")
            let code_point = symbol.chars().next().must_val() as u32;
            let binary_string = format!("{code_point:08b}");
            let mut bin_rep = BinaryRepresentation {
                character: id,
                binary_characters: Vec::new(),
                pending_binary_characters: Vec::new(),
                input_coord,
                is_active: false,
            };
            for binary_char in binary_string.chars() {
                let added = ctx
                    .terminal
                    .add_character(&binary_char.to_string(), Coord::new(0, 0));
                bin_rep.binary_characters.push(added);
                bin_rep.pending_binary_characters.push(added);
            }
            self.pending_binary_representations.push(bin_rep);
        }

        let mut pending_reps = std::mem::take(&mut self.pending_binary_representations);
        for bin_rep in &pending_reps {
            let mut path_coords: Vec<Coord> = Vec::new();
            let starting_coord = ctx.terminal.canvas.random_coord(&mut ctx.rng, true, false);
            path_coords.push(starting_coord);
            let mut last_orientation = *ctx.rng.choice(&[Orientation::Col, Orientation::Row]);
            let mut next_coord = starting_coord; // will be rebound in the loop
            let input_coord = bin_rep.input_coord;
            while *path_coords.last().must_val() != input_coord {
                let last_coord = *path_coords.last().must_val();
                let column_direction = if last_coord.column > input_coord.column {
                    -1
                } else if last_coord.column == input_coord.column {
                    0
                } else {
                    1
                };
                let row_direction = if last_coord.row > input_coord.row {
                    -1
                } else if last_coord.row == input_coord.row {
                    0
                } else {
                    1
                };
                let max_column_distance = (last_coord.column - input_coord.column).abs();
                let max_row_distance = (last_coord.row - input_coord.row).abs();
                if last_orientation == Orientation::Col && max_row_distance > 0 {
                    // min(max_row_distance, max(10, int(canvas.right * 0.2))) — int() truncation
                    let limit = std::cmp::min(
                        max_row_distance,
                        std::cmp::max(10, (ctx.terminal.canvas.right as f64 * 0.2) as i64),
                    );
                    next_coord = Coord::new(
                        last_coord.column,
                        last_coord.row + ctx.rng.randint(1, limit) * row_direction,
                    );
                    last_orientation = Orientation::Row;
                } else if last_orientation == Orientation::Row && max_column_distance > 0 {
                    next_coord = Coord::new(
                        last_coord.column
                            + ctx.rng.randint(1, std::cmp::min(max_column_distance, 4))
                                * column_direction,
                        last_coord.row,
                    );
                    last_orientation = Orientation::Col;
                } else {
                    next_coord = input_coord;
                }

                path_coords.push(next_coord);
            }

            path_coords.push(next_coord);
            let final_coord = input_coord;
            path_coords.push(final_coord);
            for &bin_effectchar in &bin_rep.binary_characters {
                let digital_path = {
                    let ch = &mut ctx.terminal.arena[bin_effectchar.0 as usize];
                    ch.motion.set_coordinate(path_coords[0]);
                    let path_id = ch
                        .motion
                        .new_path(self.config.movement_speed, None, None, 0, false, "")
                        .map_err(EngineError::Other)?;
                    let path = ch.motion.paths.get_mut(&path_id).must_val();
                    for &coord in &path_coords {
                        path.new_waypoint(coord, None, "")
                            .map_err(EngineError::Other)?;
                    }
                    path_id
                };
                ctx.activate_path(self, bin_effectchar, &digital_path);
                ctx.terminal.arena[bin_effectchar.0 as usize].layer = 1;
                let color = ctx.rng.choice(&self.config.binary_colors).clone();
                let color_scn = {
                    let ch = &mut ctx.terminal.arena[bin_effectchar.0 as usize];
                    let symbol = ch.animation.current_character_visual.symbol.clone();
                    let uses_pre = ch.uses_input_preexisting_colors;
                    let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                    ch.animation
                        .scenes
                        .get_mut(&scene_id)
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
                    scene_id
                };
                ctx.activate_scene(self, bin_effectchar, &color_scn);
            }
        }
        self.pending_binary_representations = pending_reps.drain(..).collect();
        self.setup_binarypath_scenes(ctx, &characters)?;
        self.max_active_binary_groups = std::cmp::max(
            1,
            (self.config.active_binary_groups * self.pending_binary_representations.len() as f64)
                as i64,
        );
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        self.step_scene(ctx)
    }
}
