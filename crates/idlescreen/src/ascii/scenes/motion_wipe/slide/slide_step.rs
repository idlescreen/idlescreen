// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharacterFilter, CharacterGroup, CharacterSort, ColorPair, Coord, EngineCtx, EngineError,
    ExistingColorHandling, Gradient, Slide, SlideGrouping, VisualParams,
};

impl Slide {
    pub(super) fn build_scene(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
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
        for id in characters {
            let (input_fg, input_bg, input_coord) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.animation.input_fg_color,
                    ch.animation.input_bg_color,
                    ch.input_coord,
                )
            };
            let final_colors = if dynamic {
                ColorPair::new(input_fg, input_bg)
            } else {
                ColorPair::new(
                    Some(
                        *final_gradient_mapping
                            .get(&input_coord)
                            .ok_or_else(|| EngineError::Other(String::new()))?,
                    ),
                    None,
                )
            };
            self.character_final_color_map.insert(id, final_colors);
        }

        let grouping = match self.config.grouping {
            SlideGrouping::Row => CharacterGroup::RowTopToBottom,
            SlideGrouping::Column => CharacterGroup::ColumnLeftToRight,
            SlideGrouping::Diagonal => CharacterGroup::DiagonalTopLeftToBottomRight,
        };
        let mut groups = ctx
            .terminal
            .get_characters_grouped(CharacterFilter::default(), grouping);
        for group in &groups {
            for &id in group {
                let input_coord = ctx.terminal.arena[id.0 as usize].input_coord;
                let motion = &mut ctx.terminal.arena[id.0 as usize].motion;
                motion
                    .new_path(
                        self.config.movement_speed,
                        Some(self.config.movement_easing),
                        None,
                        0,
                        false,
                        "input_path",
                    )
                    .map_err(EngineError::Other)?;
                motion
                    .paths
                    .get_mut("input_path")
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
            }
        }

        for group_index in 0..groups.len() {
            // Python's loop variable `group` keeps the original (pre-reversal) list.
            let group = groups[group_index].clone();
            match self.config.grouping {
                SlideGrouping::Row => {
                    let starting_column = if self.config.merge && group_index % 2 == 0 {
                        ctx.terminal.canvas.right + 1
                    } else {
                        groups[group_index].reverse();
                        ctx.terminal.canvas.left - 1
                    };
                    let starting_column = if self.config.reverse_direction && !self.config.merge {
                        groups[group_index].reverse();
                        ctx.terminal.canvas.right + 1
                    } else {
                        starting_column
                    };
                    for i in 0..groups[group_index].len() {
                        let id = groups[group_index][i];
                        let row = ctx.terminal.arena[id.0 as usize].input_coord.row;
                        ctx.terminal.arena[id.0 as usize]
                            .motion
                            .set_coordinate(Coord::new(starting_column, row));
                    }
                }
                SlideGrouping::Column => {
                    let starting_row = if self.config.merge && group_index % 2 == 0 {
                        ctx.terminal.canvas.bottom - 1
                    } else {
                        groups[group_index].reverse();
                        ctx.terminal.canvas.top + 1
                    };
                    let starting_row = if self.config.reverse_direction && !self.config.merge {
                        groups[group_index].reverse();
                        ctx.terminal.canvas.bottom - 1
                    } else {
                        starting_row
                    };
                    for i in 0..groups[group_index].len() {
                        let id = groups[group_index][i];
                        let column = ctx.terminal.arena[id.0 as usize].input_coord.column;
                        ctx.terminal.arena[id.0 as usize]
                            .motion
                            .set_coordinate(Coord::new(column, starting_row));
                    }
                }
                SlideGrouping::Diagonal => {}
            }
            if self.config.grouping == SlideGrouping::Diagonal {
                let last_coord = ctx.terminal.arena[group
                    .last()
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .0 as usize]
                    .input_coord;
                let distance_from_outside_bottom =
                    last_coord.row - (ctx.terminal.canvas.bottom - 1);
                let mut starting_coord = Coord::new(
                    last_coord.column - distance_from_outside_bottom,
                    last_coord.row - distance_from_outside_bottom,
                );
                if self.config.merge && group_index % 2 == 0 {
                    groups[group_index].reverse();
                    let first_coord = ctx.terminal.arena[group[0].0 as usize].input_coord;
                    let distance_from_outside = (ctx.terminal.canvas.top + 1) - first_coord.row;
                    starting_coord = Coord::new(
                        first_coord.column + distance_from_outside,
                        first_coord.row + distance_from_outside,
                    );
                }
                if self.config.reverse_direction && !self.config.merge {
                    groups[group_index].reverse();
                    let first_coord = ctx.terminal.arena[group[0].0 as usize].input_coord;
                    let distance_from_outside = (ctx.terminal.canvas.top + 1) - first_coord.row;
                    starting_coord = Coord::new(
                        first_coord.column + distance_from_outside,
                        first_coord.row + distance_from_outside,
                    );
                }
                for i in 0..groups[group_index].len() {
                    let id = groups[group_index][i];
                    ctx.terminal.arena[id.0 as usize]
                        .motion
                        .set_coordinate(starting_coord);
                }
            }
            for &id in &group {
                let (input_symbol, uses_pre) = {
                    let ch = &ctx.terminal.arena[id.0 as usize];
                    (ch.input_symbol.clone(), ch.uses_input_preexisting_colors)
                };
                let final_colors = *self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let gradient_scn = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation.new_scene(false, None, None, "", uses_pre)
                };
                {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let scene = ch
                        .animation
                        .scenes
                        .get_mut(&gradient_scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    if dynamic {
                        scene
                            .add_frame(
                                &input_symbol,
                                self.config.final_gradient_frames,
                                VisualParams {
                                    colors: Some(final_colors),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    } else {
                        let final_fg_color = final_colors
                            .fg_color
                            .ok_or_else(|| EngineError::Other(String::new()))?;
                        let char_gradient = Gradient::with_steps(
                            &[self.config.final_gradient_stops[0], final_fg_color],
                            10,
                            false,
                        )
                        .map_err(EngineError::Other)?;
                        scene
                            .apply_gradient_to_symbols(
                                std::slice::from_ref(&input_symbol),
                                self.config.final_gradient_frames,
                                Some(&char_gradient),
                                None,
                            )
                            .map_err(EngineError::Other)?;
                    }
                }
                ctx.activate_scene(self, id, &gradient_scn);
            }
        }

        self.pending_groups = groups;
        self.active_groups = Vec::new();
        self.current_gap = 0;
        Ok(())
    }
}
