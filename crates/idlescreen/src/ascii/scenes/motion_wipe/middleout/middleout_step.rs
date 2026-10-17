// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharId, CharacterFilter, CharacterSort, ColorPair, Coord, Effect, EngineCtx, EngineError,
    ExistingColorHandling, ExpandDirection, Gradient, Middleout, MiddleoutConfig, Phase,
    VisualParams, emit_frame,
};

impl Effect for Middleout {
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
        for id in characters {
            let (input_fg, input_bg, input_coord, input_symbol, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.animation.input_fg_color,
                    ch.animation.input_bg_color,
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.uses_input_preexisting_colors,
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
            let center = ctx.terminal.canvas.center;
            ctx.terminal.arena[id.0 as usize]
                .motion
                .set_coordinate(center);
            // setup waypoints
            let (column, row) = match self.config.expand_direction {
                ExpandDirection::Vertical => (input_coord.column, ctx.terminal.canvas.center_row),
                ExpandDirection::Horizontal => (ctx.terminal.canvas.center_column, input_coord.row),
            };
            let center_path = {
                let motion = &mut ctx.terminal.arena[id.0 as usize].motion;
                let path_id = motion
                    .new_path(
                        self.config.center_movement_speed,
                        Some(self.config.center_easing),
                        None,
                        0,
                        false,
                        "",
                    )
                    .map_err(EngineError::Other)?;
                motion
                    .paths
                    .get_mut(&path_id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(Coord::new(column, row), None, "")
                    .map_err(EngineError::Other)?;
                path_id
            };
            {
                let motion = &mut ctx.terminal.arena[id.0 as usize].motion;
                motion
                    .new_path(
                        self.config.full_movement_speed,
                        Some(self.config.full_easing),
                        None,
                        0,
                        false,
                        "full",
                    )
                    .map_err(EngineError::Other)?;
                motion
                    .paths
                    .get_mut("full")
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(input_coord, None, "full")
                    .map_err(EngineError::Other)?;
            }

            // setup scenes
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "full", uses_pre);
                let scene = ch
                    .animation
                    .scenes
                    .get_mut("full")
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let final_fg_color = final_colors.fg_color;
                let final_bg_color = final_colors.bg_color;
                if dynamic {
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
                    let final_fg_color =
                        final_fg_color.ok_or_else(|| EngineError::Other(String::new()))?;
                    let full_gradient = Gradient::with_steps(
                        &[self.config.starting_color, final_fg_color],
                        10,
                        false,
                    )
                    .map_err(EngineError::Other)?;
                    scene
                        .apply_gradient_to_symbols(
                            std::slice::from_ref(&input_symbol),
                            6,
                            Some(&full_gradient),
                            None,
                        )
                        .map_err(EngineError::Other)?;
                }
            }

            // initialize character state
            ctx.activate_path(self, id, &center_path);
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let input_symbol = ch.input_symbol.clone();
                let uses_pre = ch.uses_input_preexisting_colors;
                ch.animation.set_appearance(
                    &input_symbol,
                    uses_pre,
                    Some(&input_symbol.clone()),
                    Some(ColorPair::new(Some(self.config.starting_color), None)),
                );
            }
            ctx.terminal.set_character_visibility(id, true);
            ctx.active_characters.insert(id);
        }
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.phase == Phase::Center && ctx.active_characters.is_empty() {
            self.phase = Phase::Full;
            let characters = {
                let filter = CharacterFilter::default();
                ctx.terminal.get_characters(
                    &mut ctx.rng,
                    filter,
                    CharacterSort::TopToBottomLeftToRight,
                )
            };
            ctx.active_characters = characters.into_iter().collect();
            // upstream iterates the set here; canonical ascending character_id
            let ordered: Vec<CharId> = ctx.active_characters.iter().collect();
            for id in ordered {
                ctx.activate_path(self, id, "full");
                ctx.activate_scene(self, id, "full");
            }
        }
        if !ctx.active_characters.is_empty() {
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Middleout {
    fn default() -> Self {
        Self::new(MiddleoutConfig::default())
    }
}
