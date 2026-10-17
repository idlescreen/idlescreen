// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CallerKey, CharId, CharacterFilter, CharacterGroup, CharacterSort, ColorPair, Coord, Effect,
    EngineCtx, EngineError, Event, EventAction, ExistingColorHandling, Gradient, Launcher,
    OrbittingVolley, OrbittingVolleyConfig, emit_frame,
};

impl OrbittingVolley {
    pub(super) fn build_scene(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let final_gradient = Gradient::new(
            &self.config.final_gradient_stops,
            &self.config.final_gradient_steps,
            false,
            false,
        )
        .map_err(EngineError::Other)?;
        let final_gradient_coordinate_map = final_gradient
            .build_coordinate_color_mapping(
                ctx.terminal.canvas.text_bottom,
                ctx.terminal.canvas.text_top,
                ctx.terminal.canvas.text_left,
                ctx.terminal.canvas.text_right,
                self.config.final_gradient_direction,
            )
            .map_err(EngineError::Other)?;
        self.launcher_gradient_coordinate_map = final_gradient
            .build_coordinate_color_mapping(
                ctx.terminal.canvas.bottom,
                ctx.terminal.canvas.top,
                ctx.terminal.canvas.left,
                ctx.terminal.canvas.right,
                self.config.final_gradient_direction,
            )
            .map_err(EngineError::Other)?;
        self.final_gradient_last_color = Some(
            *final_gradient
                .spectrum
                .last()
                .ok_or_else(|| EngineError::Other(String::new()))?,
        );

        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            let (input_coord, input_symbol, input_fg, input_bg, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.animation.input_fg_color,
                    ch.animation.input_bg_color,
                    ch.uses_input_preexisting_colors,
                )
            };
            let final_colors = if dynamic {
                ColorPair::new(input_fg, input_bg)
            } else {
                ColorPair::new(
                    Some(
                        *final_gradient_coordinate_map
                            .get(&input_coord)
                            .ok_or_else(|| EngineError::Other(String::new()))?,
                    ),
                    None,
                )
            };
            self.character_final_color_map.insert(id, final_colors);
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let input_path = ch
                    .motion
                    .new_path(
                        self.config.character_movement_speed,
                        Some(self.config.character_easing),
                        Some(1),
                        0,
                        false,
                        "input_path",
                    )
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&input_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
            }
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path("input_path".to_string()),
                EventAction::SetLayer(0),
            )
            .map_err(EngineError::Other)?;
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation.set_appearance(
                &input_symbol,
                uses_pre,
                Some(&input_symbol.clone()),
                Some(final_colors),
            );
        }

        let launcher_specs = [
            (
                Coord::new(ctx.terminal.canvas.left, ctx.terminal.canvas.top),
                self.config.top_launcher_symbol.clone(),
            ),
            (
                Coord::new(ctx.terminal.canvas.right, ctx.terminal.canvas.top),
                self.config.right_launcher_symbol.clone(),
            ),
            (
                Coord::new(ctx.terminal.canvas.right, ctx.terminal.canvas.bottom),
                self.config.bottom_launcher_symbol.clone(),
            ),
            (
                Coord::new(ctx.terminal.canvas.left, ctx.terminal.canvas.bottom),
                self.config.left_launcher_symbol.clone(),
            ),
        ];
        for (coord, symbol) in launcher_specs {
            let character = ctx.terminal.add_character(&symbol, coord);
            ctx.terminal.arena[character.0 as usize].layer = 2;
            ctx.terminal.set_character_visibility(character, true);
            ctx.active_characters.insert(character);
            self.launchers.push(Launcher {
                character,
                magazine: Vec::new(),
            });
        }
        let main_character = self.launchers[0].character;
        {
            let color = self.final_gradient_last_color;
            let ch = &mut ctx.terminal.arena[main_character.0 as usize];
            let input_symbol = ch.input_symbol.clone();
            let uses_pre = ch.uses_input_preexisting_colors;
            ch.animation.set_appearance(
                &input_symbol,
                uses_pre,
                Some(&input_symbol.clone()),
                Some(ColorPair::new(color, None)),
            );
        }
        self.build_launcher_paths(ctx, main_character)?;
        ctx.activate_path(self, main_character, "perimeter");

        let mut sorted_chars: Vec<CharId> = Vec::new();
        for char_list in ctx
            .terminal
            .get_characters_grouped(CharacterFilter::default(), CharacterGroup::CenterToOutside)
        {
            sorted_chars.extend(char_list);
        }
        for (index, character) in sorted_chars.into_iter().enumerate() {
            let launcher_index = index % self.launchers.len();
            self.launchers[launcher_index].magazine.push(character);
        }
        self.delay = 0;
        Ok(())
    }
}
impl Effect for OrbittingVolley {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.build_scene(ctx)
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.launchers.iter().any(|l| !l.magazine.is_empty()) || ctx.active_characters.len() > 1
        {
            let main_character = self.launchers[0].character;
            if ctx.terminal.arena[main_character.0 as usize]
                .motion
                .active_path
                .is_none()
            {
                let first_waypoint_coord = ctx.terminal.arena[main_character.0 as usize]
                    .motion
                    .paths
                    .get("perimeter")?
                    .waypoints[0]
                    .coord;
                ctx.terminal.arena[main_character.0 as usize]
                    .motion
                    .set_coordinate(first_waypoint_coord);
                ctx.activate_path(self, main_character, "perimeter");
                ctx.active_characters.insert(main_character);
            }
            {
                let current_coord = ctx.terminal.arena[main_character.0 as usize]
                    .motion
                    .current_coord;
                let color = *self.launcher_gradient_coordinate_map.get(&current_coord)?;
                let symbol = self.config.top_launcher_symbol.clone();
                let ch = &mut ctx.terminal.arena[main_character.0 as usize];
                let input_symbol = ch.input_symbol.clone();
                let uses_pre = ch.uses_input_preexisting_colors;
                ch.animation.set_appearance(
                    &input_symbol,
                    uses_pre,
                    Some(&symbol),
                    Some(ColorPair::new(Some(color), None)),
                );
            }
            for child_index in 1..self.launchers.len() {
                self.set_launcher_coordinates(ctx, 0, child_index);
            }
            if self.delay == 0 {
                for launcher_index in 0..self.launchers.len() {
                    // max(int((volley_size * len(input_characters)) / 4), 1) — int() truncation
                    let characters_to_launch = std::cmp::max(
                        ((self.config.volley_size * ctx.terminal.input_characters.len() as f64)
                            / 4.0) as i64,
                        1,
                    );
                    for _ in 0..characters_to_launch {
                        if let Some(next_char) = self.launch(ctx, launcher_index) {
                            ctx.active_characters.insert(next_char);
                        }
                    }
                }
                self.delay = self.config.launch_delay;
            } else {
                self.delay -= 1;
            }

            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        if !self.complete {
            self.complete = true;
            for launcher_index in 0..self.launchers.len() {
                let character = self.launchers[launcher_index].character;
                ctx.terminal.set_character_visibility(character, false);
            }
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for OrbittingVolley {
    fn default() -> Self {
        Self::new(OrbittingVolleyConfig::default())
    }
}
