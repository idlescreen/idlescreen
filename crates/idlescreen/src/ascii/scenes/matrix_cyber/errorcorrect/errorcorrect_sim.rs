// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharId, CharacterFilter, CharacterSort, ColorPair, Effect, EngineCtx, EngineError,
    ErrorCorrect, ErrorCorrectConfig, ExistingColorHandling, Gradient, VisualParams, emit_frame,
};

impl Effect for ErrorCorrect {
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
        for &id in &characters {
            let spawn_colors = *self
                .character_final_color_map
                .get(&id)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            let spawn_scene = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let uses_pre = ch.uses_input_preexisting_colors;
                let input_symbol = ch.input_symbol.clone();
                let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                ch.animation
                    .scenes
                    .get_mut(&scene_id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(spawn_colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                scene_id
            };
            ctx.activate_scene(self, id, &spawn_scene);
            ctx.terminal.set_character_visibility(id, true);
        }

        let mut all_characters: Vec<CharId> = ctx.terminal.input_characters.clone();
        let correcting_gradient = Gradient::with_steps(
            &[self.config.error_color, self.config.correct_color],
            10,
            false,
        )
        .map_err(EngineError::Other)?;

        let pair_count = (self.config.error_pairs * characters.len() as f64) as i64;
        for _ in 0..pair_count {
            if all_characters.len() < 2 {
                break;
            }
            let index1 = ctx.rng.randrange(0, all_characters.len() as i64) as usize;
            let char1 = all_characters.remove(index1);
            let index2 = ctx.rng.randrange(0, all_characters.len() as i64) as usize;
            let char2 = all_characters.remove(index2);
            let char1_input_coord = ctx.terminal.arena[char1.0 as usize].input_coord;
            let char2_input_coord = ctx.terminal.arena[char2.0 as usize].input_coord;
            {
                let ch = &mut ctx.terminal.arena[char1.0 as usize];
                ch.motion.set_coordinate(char2_input_coord);
                let path_id = ch
                    .motion
                    .new_path(
                        self.config.movement_speed,
                        None,
                        None,
                        0,
                        false,
                        "input_coord",
                    )
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(char1_input_coord, None, "")
                    .map_err(EngineError::Other)?;
            }
            {
                let ch = &mut ctx.terminal.arena[char2.0 as usize];
                ch.motion.set_coordinate(char1_input_coord);
                let path_id = ch
                    .motion
                    .new_path(
                        self.config.movement_speed,
                        None,
                        None,
                        0,
                        false,
                        "input_coord",
                    )
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(char2_input_coord, None, "")
                    .map_err(EngineError::Other)?;
            }
            self.swapped.push((char1, char2));
            for id in [char1, char2] {
                self.configure_swapped_character(ctx, id, &correcting_gradient)?;
            }
        }
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.swapped.is_empty() && self.swap_delay == 0 {
            let (char1, char2) = self.swapped.remove(0);
            for id in [char1, char2] {
                ctx.activate_scene(self, id, "error");
                ctx.active_characters.insert(id);
            }
            self.swap_delay = self.config.swap_delay;
        } else if self.swap_delay != 0 {
            self.swap_delay -= 1;
        }
        if !ctx.active_characters.is_empty() {
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for ErrorCorrect {
    fn default() -> Self {
        Self::new(ErrorCorrectConfig::default())
    }
}
