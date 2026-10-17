// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CallerKey, CharacterFilter, CharacterSort, ColorPair, Coord, Effect, EngineCtx, EngineError,
    Event, EventAction, ExistingColorHandling, Gradient, Rain, RainConfig, VisualParams,
    emit_frame,
};

impl Effect for Rain {
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
                    ColorPair::new(ch.animation.input_fg_color, ch.animation.input_bg_color)
                } else {
                    ColorPair::new(
                        Some(
                            *final_gradient_mapping
                                .get(&ch.input_coord)
                                .ok_or_else(|| EngineError::Other(String::new()))?,
                        ),
                        None,
                    )
                }
            };
            self.character_final_color_map.insert(id, final_colors);
        }

        let canvas_top = ctx.terminal.canvas.top;
        for id in characters {
            let (input_coord, input_symbol, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            let raindrop_color = *ctx.rng.choice(&self.config.rain_colors);
            let rain_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "", uses_pre)
            };
            let rain_symbol = ctx.rng.choice(&self.config.rain_symbols).clone();
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&rain_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        &rain_symbol,
                        1,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(raindrop_color), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            let fade_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "", uses_pre)
            };
            let final_colors = *self
                .character_final_color_map
                .get(&id)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            if dynamic {
                let fg_gradient = match &final_colors.fg_color {
                    Some(fg) => Some(
                        Gradient::with_steps(&[raindrop_color, *fg], 7, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let bg_gradient = match &final_colors.bg_color {
                    Some(bg) => Some(
                        Gradient::with_steps(&[raindrop_color, *bg], 7, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene = ch
                    .animation
                    .scenes
                    .get_mut(&fade_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                if fg_gradient.is_some() || bg_gradient.is_some() {
                    scene
                        .apply_gradient_to_symbols(
                            std::slice::from_ref(&input_symbol),
                            3,
                            fg_gradient.as_ref(),
                            bg_gradient.as_ref(),
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
            } else {
                let final_fg = final_colors
                    .fg_color
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let raindrop_gradient = Gradient::with_steps(&[raindrop_color, final_fg], 7, false)
                    .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&fade_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .apply_gradient_to_symbols(
                        std::slice::from_ref(&input_symbol),
                        3,
                        Some(&raindrop_gradient),
                        None,
                    )
                    .map_err(EngineError::Other)?;
            }
            ctx.activate_scene(self, id, &rain_scn);
            let speed = ctx
                .rng
                .uniform(self.config.movement_speed.0, self.config.movement_speed.1);
            let input_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .set_coordinate(Coord::new(input_coord.column, canvas_top));
                let path_id = ch
                    .motion
                    .new_path(speed, Some(self.config.movement_easing), None, 0, false, "")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
                path_id
            };
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(input_path.clone()),
                EventAction::ActivateScene(fade_scn),
            )
            .map_err(EngineError::Other)?;
            ctx.activate_path(self, id, &input_path);
            self.pending_chars.push(id);
        }
        let mut sorted_chars = self.pending_chars.clone();
        sorted_chars.sort_by_key(|&id| ctx.terminal.arena[id.0 as usize].input_coord.row);
        for id in sorted_chars {
            let row = ctx.terminal.arena[id.0 as usize].input_coord.row;
            self.group_by_row.entry(row).or_default().push(id);
        }
        self.pending_chars.clear();
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.group_by_row.is_empty()
            || !ctx.active_characters.is_empty()
            || !self.pending_chars.is_empty()
        {
            if self.pending_chars.is_empty() && !self.group_by_row.is_empty() {
                let min_row = *self.group_by_row.keys().next()?;
                let group = self.group_by_row.remove(&min_row)?;
                self.pending_chars.extend(group);
            }
            if !self.pending_chars.is_empty() {
                for _ in 0..ctx.rng.randint(1, 2) {
                    if self.pending_chars.is_empty() {
                        break;
                    }
                    let index = ctx.rng.randint(0, self.pending_chars.len() as i64 - 1) as usize;
                    let next_character = self.pending_chars.remove(index);
                    ctx.terminal.set_character_visibility(next_character, true);
                    ctx.active_characters.insert(next_character);
                }
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Rain {
    fn default() -> Self {
        Self::new(RainConfig::default())
    }
}
