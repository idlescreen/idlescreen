// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharacterFilter, CharacterGroup, CharacterSort, ColorPair, Coord, Effect, EngineCtx,
    EngineError, ExistingColorHandling, Gradient, Pour, PourConfig, PourDirection, VisualParams,
    emit_frame,
};

impl Effect for Pour {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let pour_direction = self.config.pour_direction;
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
        let grouping = match pour_direction {
            PourDirection::Down => CharacterGroup::RowBottomToTop,
            PourDirection::Up => CharacterGroup::RowTopToBottom,
            PourDirection::Left => CharacterGroup::ColumnLeftToRight,
            PourDirection::Right => CharacterGroup::ColumnRightToLeft,
        };
        let groups = ctx
            .terminal
            .get_characters_grouped(CharacterFilter::default(), grouping);
        for (i, group) in groups.iter().enumerate() {
            for &id in group {
                ctx.terminal.set_character_visibility(id, false);
                let (input_coord, input_symbol, uses_pre) = {
                    let ch = &ctx.terminal.arena[id.0 as usize];
                    (
                        ch.input_coord,
                        ch.input_symbol.clone(),
                        ch.uses_input_preexisting_colors,
                    )
                };
                let start_coord = match pour_direction {
                    PourDirection::Down => Coord::new(input_coord.column, ctx.terminal.canvas.top),
                    PourDirection::Up => Coord::new(input_coord.column, ctx.terminal.canvas.bottom),
                    PourDirection::Left => Coord::new(ctx.terminal.canvas.right, input_coord.row),
                    PourDirection::Right => Coord::new(ctx.terminal.canvas.left, input_coord.row),
                };
                ctx.terminal.arena[id.0 as usize]
                    .motion
                    .set_coordinate(start_coord);
                let speed = ctx.rng.uniform(
                    self.config.movement_speed_range.0,
                    self.config.movement_speed_range.1,
                );
                let input_coord_path = ctx.terminal.arena[id.0 as usize]
                    .motion
                    .new_path(speed, Some(self.config.movement_easing), None, 0, false, "")
                    .map_err(EngineError::Other)?;
                ctx.terminal.arena[id.0 as usize]
                    .motion
                    .paths
                    .get_mut(&input_coord_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
                ctx.activate_path(self, id, &input_coord_path);

                let pour_scn = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation.new_scene(false, None, None, "", uses_pre)
                };
                let final_colors = *self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let scene = ch
                        .animation
                        .scenes
                        .get_mut(&pour_scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    if dynamic {
                        let final_fg_color = final_colors.fg_color;
                        let final_bg_color = final_colors.bg_color;
                        let fg_gradient = match &final_fg_color {
                            Some(c) => Some(
                                Gradient::with_steps(&[self.config.starting_color, *c], 10, false)
                                    .map_err(EngineError::Other)?,
                            ),
                            None => None,
                        };
                        let bg_gradient = match &final_bg_color {
                            Some(c) => Some(
                                Gradient::with_steps(&[self.config.starting_color, *c], 10, false)
                                    .map_err(EngineError::Other)?,
                            ),
                            None => None,
                        };
                        if fg_gradient.is_some() || bg_gradient.is_some() {
                            scene
                                .apply_gradient_to_symbols(
                                    std::slice::from_ref(&input_symbol),
                                    self.config.final_gradient_frames,
                                    fg_gradient.as_ref(),
                                    bg_gradient.as_ref(),
                                )
                                .map_err(EngineError::Other)?;
                        } else {
                            scene
                                .add_frame(
                                    &input_symbol,
                                    self.config.final_gradient_frames,
                                    VisualParams {
                                        colors: Some(ColorPair::default()),
                                        ..Default::default()
                                    },
                                )
                                .map_err(EngineError::Other)?;
                        }
                    } else {
                        let final_fg_color = final_colors
                            .fg_color
                            .ok_or_else(|| EngineError::Other(String::new()))?;
                        let pour_gradient = Gradient::new(
                            &[self.config.starting_color, final_fg_color],
                            &self.config.final_gradient_steps,
                            false,
                            false,
                        )
                        .map_err(EngineError::Other)?;
                        scene
                            .apply_gradient_to_symbols(
                                std::slice::from_ref(&input_symbol),
                                self.config.final_gradient_frames,
                                Some(&pour_gradient),
                                None,
                            )
                            .map_err(EngineError::Other)?;
                    }
                }
                ctx.activate_scene(self, id, &pour_scn);
            }
            if i % 2 == 0 {
                self.pending_groups.push(group.clone());
            } else {
                let mut reversed = group.clone();
                reversed.reverse();
                self.pending_groups.push(reversed);
            }
        }
        self.gap = 0;
        self.current_group = self.pending_groups.remove(0);
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.pending_groups.is_empty()
            || !ctx.active_characters.is_empty()
            || !self.current_group.is_empty()
        {
            if self.current_group.is_empty() && !self.pending_groups.is_empty() {
                self.current_group = self.pending_groups.remove(0);
            }
            if !self.current_group.is_empty() {
                if self.gap == 0 {
                    for _ in 0..self.config.pour_speed {
                        if !self.current_group.is_empty() {
                            let next_character = self.current_group.remove(0);
                            ctx.terminal.set_character_visibility(next_character, true);
                            ctx.active_characters.insert(next_character);
                        }
                    }
                    self.gap = self.config.gap;
                } else {
                    self.gap -= 1;
                }
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Pour {
    fn default() -> Self {
        Self::new(PourConfig::default())
    }
}
