// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CB_SET_INVISIBLE, CallerKey, CharId, CharacterFilter, CharacterSort, ColorPair, Easing,
    EffectCallback, EngineCtx, EngineError, Event, EventAction, ExistingColorHandling, Gradient,
    HashSet, Phase, Ring, Rings, VisualParams, geometry, round_half_even,
};

impl Rings {
    pub(super) fn build_scene(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // ring_gap = int(max(round(min(top, right) * config.ring_gap), 1))
        let ring_gap = std::cmp::max(
            round_half_even(
                std::cmp::min(ctx.terminal.canvas.top, ctx.terminal.canvas.right) as f64
                    * self.config.ring_gap,
            ),
            1,
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
        let mut pending_chars: Vec<CharId> = Vec::new();
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
                        *final_gradient_mapping
                            .get(&input_coord)
                            .ok_or_else(|| EngineError::Other(String::new()))?,
                    ),
                    None,
                )
            };
            self.character_final_color_map.insert(id, final_colors);
            let start_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                ch.animation
                    .scenes
                    .get_mut(&scene_id)
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
                scene_id
            };
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let home_path = ch
                    .motion
                    .new_path(0.8, Some(Easing::OutQuad), None, 0, false, "home")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&home_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
            }
            ctx.activate_scene(self, id, &start_scn);
            ctx.terminal.set_character_visibility(id, true);
            pending_chars.push(id);
        }

        ctx.rng.shuffle(&mut pending_chars);

        // make rings
        let mut rings: Vec<Ring> = Vec::new();
        let center = ctx.terminal.canvas.center;
        let radius_limit = std::cmp::max(ctx.terminal.canvas.right, ctx.terminal.canvas.top);
        let mut radius = 1;
        while radius < radius_limit {
            let ring_coords = geometry::find_coords_on_circle(center, radius, 7 * radius, true);
            // check if any part of the ring is in the canvas, if not, stop creating rings
            let in_canvas_count = ring_coords
                .iter()
                .filter(|&&coord| ctx.terminal.canvas.coord_is_in_canvas(coord))
                .count();
            if (in_canvas_count as f64) / (ring_coords.len() as f64) < 0.25 {
                break;
            }
            let ring_color = self.config.ring_colors[rings.len() % self.config.ring_colors.len()];
            let ring = Ring::new(ctx, &self.config, ring_coords, ring_gap, ring_color);
            rings.push(ring);
            radius += ring_gap;
        }

        // assign characters to rings
        let mut pending_iter = pending_chars
            .into_iter()
            .collect::<std::collections::VecDeque<_>>();
        let mut ring_chars: HashSet<CharId> = HashSet::new();
        for (ring_count, ring) in rings.iter_mut().enumerate() {
            for _ in 0..ring.counter_clockwise_coords.len() {
                if let Some(next_character) = pending_iter.pop_front() {
                    // set rings to rotate in opposite directions
                    let clockwise = ring_count % 2 == 1;
                    self.ring_add_character(ctx, ring, next_character, clockwise)?;
                    ring_chars.insert(next_character);
                }
            }
        }
        self.rings = rings;

        // make external waypoints for characters not in rings
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            if ring_chars.contains(&id) {
                continue;
            }
            let external_coord = ctx.terminal.canvas.random_coord(&mut ctx.rng, true, false);
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let external_path = ch
                    .motion
                    .new_path(0.8, Some(Easing::OutSine), None, 0, false, "external")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&external_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(external_coord, None, "")
                    .map_err(EngineError::Other)?;
            }
            self.non_ring_chars.push(id);
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path("external".to_string()),
                EventAction::Callback(EffectCallback {
                    id: CB_SET_INVISIBLE,
                    args: Vec::new(),
                }),
            )
            .map_err(EngineError::Other)?;
        }
        self.phase = Phase::Start;
        self.initial_disperse_complete = false;
        self.spin_time_remaining = self.config.spin_duration;
        self.disperse_time_remaining = self.config.disperse_duration;
        self.cycles_remaining = self.config.spin_disperse_cycles;
        self.initial_phase_time_remaining = 100;
        Ok(())
    }
}
