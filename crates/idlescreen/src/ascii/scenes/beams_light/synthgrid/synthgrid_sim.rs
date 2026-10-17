// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CB_UPDATE_GROUP_TRACKER, CallbackValue, CallerKey, CharId, CharacterFilter, CharacterSort,
    ColorPair, Coord, Direction, EffectCallback, EngineCtx, EngineError, Event, EventAction,
    ExistingColorHandling, Gradient, Phase, SynthGrid, VisualParams, floor_div,
};

impl SynthGrid {
    pub(super) fn build_scene(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let grid_gradient = Gradient::new(
            &self.config.grid_gradient_stops,
            &self.config.grid_gradient_steps,
            false,
            false,
        )
        .map_err(EngineError::Other)?;
        let grid_gradient_mapping = grid_gradient
            .build_coordinate_color_mapping(
                1,
                ctx.terminal.canvas.top,
                1,
                ctx.terminal.canvas.right,
                self.config.grid_gradient_direction,
            )
            .map_err(EngineError::Other)?;
        let text_gradient = Gradient::new(
            &self.config.text_gradient_stops,
            &self.config.text_gradient_steps,
            false,
            false,
        )
        .map_err(EngineError::Other)?;
        let text_gradient_mapping = text_gradient
            .build_coordinate_color_mapping(
                ctx.terminal.canvas.text_bottom,
                ctx.terminal.canvas.text_top,
                ctx.terminal.canvas.text_left,
                ctx.terminal.canvas.text_right,
                self.config.text_gradient_direction,
            )
            .map_err(EngineError::Other)?;
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = ctx.terminal.get_characters(
            &mut ctx.rng,
            CharacterFilter::default(),
            CharacterSort::TopToBottomLeftToRight,
        );
        for id in characters {
            let ch = &ctx.terminal.arena[id.0 as usize];
            let colors = if dynamic {
                ColorPair::new(ch.animation.input_fg_color, ch.animation.input_bg_color)
            } else if ch.input_symbol != " " {
                ColorPair::new(
                    Some(
                        *text_gradient_mapping
                            .get(&ch.input_coord)
                            .ok_or_else(|| EngineError::Other(String::new()))?,
                    ),
                    None,
                )
            } else {
                ColorPair::default()
            };
            self.character_final_color_map.insert(id, colors);
        }

        let (canvas_left, canvas_right, canvas_bottom, canvas_top) = {
            let c = &ctx.terminal.canvas;
            (c.left, c.right, c.bottom, c.top)
        };
        for (coord, dir) in [
            (
                Coord::new(canvas_left, canvas_bottom),
                Direction::Horizontal,
            ),
            (Coord::new(canvas_left, canvas_top), Direction::Horizontal),
            (Coord::new(canvas_left, canvas_bottom), Direction::Vertical),
            (Coord::new(canvas_right, canvas_bottom), Direction::Vertical),
        ] {
            let line = self.make_grid_line(ctx, coord, dir, &grid_gradient_mapping)?;
            self.grid_lines.push(line);
        }

        let mut column_indexes: Vec<i64> = Vec::new();
        let mut row_indexes: Vec<i64> = Vec::new();
        let (row_gap, column_gap) = if canvas_top > 2 * canvas_right {
            let row_gap = Self::find_even_gap(canvas_top) + 1;
            (row_gap, row_gap * 2)
        } else {
            let column_gap = Self::find_even_gap(canvas_right) + 1;
            (floor_div(column_gap, 2), column_gap)
        };

        // range(bottom + row_gap, top, max(row_gap, 1))
        let row_step = std::cmp::max(row_gap, 1);
        let mut row_index = canvas_bottom + row_gap;
        while row_index < canvas_top {
            if canvas_top - row_index >= 2 {
                row_indexes.push(row_index);
                let line = self.make_grid_line(
                    ctx,
                    Coord::new(canvas_left, row_index),
                    Direction::Horizontal,
                    &grid_gradient_mapping,
                )?;
                self.grid_lines.push(line);
            }
            row_index += row_step;
        }
        // range(left + column_gap, right, max(column_gap, 1))
        let column_step = std::cmp::max(column_gap, 1);
        let mut column_index = canvas_left + column_gap;
        while column_index < canvas_right {
            if canvas_right - column_index >= 2 {
                column_indexes.push(column_index);
                let line = self.make_grid_line(
                    ctx,
                    Coord::new(column_index, canvas_bottom),
                    Direction::Vertical,
                    &grid_gradient_mapping,
                )?;
                self.grid_lines.push(line);
            }
            column_index += column_step;
        }
        row_indexes.push(canvas_top + 1);
        column_indexes.push(canvas_right + 1);
        let mut prev_row_index = 1i64;
        for &row_index_value in &row_indexes {
            // row_index is reassigned inside the column loop (noqa: PLW2901
            // upstream) and the mutated value flows into prev_row_index.
            let mut row_index = row_index_value;
            let mut prev_column_index = 1i64;
            for &column_index in &column_indexes {
                let mut coords_in_block: Vec<Coord> = Vec::new();
                if row_index == canvas_top {
                    // make sure the top row is included
                    row_index += 1;
                }
                for row in prev_row_index..row_index {
                    for column in prev_column_index..column_index {
                        coords_in_block.push(Coord::new(column, row));
                    }
                }
                let mut characters_in_block: Vec<CharId> = Vec::new();
                for coord in &coords_in_block {
                    if let Some(&id) = ctx.terminal.character_by_input_coord.get(coord) {
                        characters_in_block.push(id);
                    }
                }
                if !characters_in_block.is_empty() {
                    self.pending_groups
                        .push((self.pending_groups.len() as i64, characters_in_block));
                }
                prev_column_index = column_index;
            }
            prev_row_index = row_index;
        }

        self.group_tracker = vec![0; self.pending_groups.len()];
        let pending_groups = std::mem::take(&mut self.pending_groups);
        for (group_number, group) in &pending_groups {
            for &character in group {
                let (input_symbol, uses_pre) = {
                    let ch = &ctx.terminal.arena[character.0 as usize];
                    (ch.input_symbol.clone(), ch.uses_input_preexisting_colors)
                };
                let dissolve_scn = ctx.terminal.arena[character.0 as usize]
                    .animation
                    .new_scene(false, None, None, "", uses_pre);
                let frame_count = ctx.rng.randint(8, 16);
                for _ in 0..frame_count {
                    let symbol = ctx.rng.choice(&self.config.text_generation_symbols).clone();
                    let fg = *ctx.rng.choice(&text_gradient.spectrum);
                    ctx.terminal.arena[character.0 as usize]
                        .animation
                        .scenes
                        .get_mut(&dissolve_scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?
                        .add_frame(
                            &symbol,
                            2,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(fg), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                let final_colors = self
                    .character_final_color_map
                    .get(&character)
                    .copied()
                    .unwrap_or_default();
                ctx.terminal.arena[character.0 as usize]
                    .animation
                    .scenes
                    .get_mut(&dissolve_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(final_colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                ctx.activate_scene(self, character, &dissolve_scn);
                ctx.register_event(
                    character,
                    Event::SceneComplete,
                    CallerKey::Scene(dissolve_scn.clone()),
                    EventAction::Callback(EffectCallback {
                        id: CB_UPDATE_GROUP_TRACKER,
                        args: vec![CallbackValue::Int(*group_number)],
                    }),
                )
                .map_err(EngineError::Other)?;
            }
        }
        self.pending_groups = pending_groups;
        ctx.rng.shuffle(&mut self.pending_groups);
        self.phase = Phase::GridExpand;
        self.total_group_count = self.pending_groups.len();
        if self.total_group_count == 0 {
            let characters = ctx.terminal.get_characters(
                &mut ctx.rng,
                CharacterFilter::default(),
                CharacterSort::TopToBottomLeftToRight,
            );
            for character in characters {
                ctx.terminal.set_character_visibility(character, true);
                ctx.active_characters.insert(character);
            }
        }
        self.active_groups = 0;
        Ok(())
    }
}
