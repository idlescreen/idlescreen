// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharacterFilter, CharacterSort, Color, ColorPair, Coord, Effect, EngineCtx, EngineError,
    ExistingColorHandling, Gradient, HashMap, Swarm, floor_div, geometry, hex_color,
    round_half_even,
};

impl Effect for Swarm {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // SwarmIterator.DYNAMIC_CLEAR_COLOR
        let dynamic_clear_color = hex_color("#ffffff");
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        let swarm_size: i64 = std::cmp::max(
            round_half_even(characters.len() as f64 * self.config.swarm_size),
            1,
        );
        self.make_swarms(ctx, swarm_size);
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
        for &id in &characters {
            let ch = &ctx.terminal.arena[id.0 as usize];
            let final_colors = if dynamic {
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
            };
            self.character_final_color_map.insert(id, final_colors);
        }
        let flash_list: Vec<Color> = (0..10).map(|_| self.config.flash_color).collect();
        let canvas_right = ctx.terminal.canvas.right;
        let canvas_top = ctx.terminal.canvas.top;
        // stands in for upstream's process-wide find_coords_on_circle lru_cache
        let mut circle_cache: HashMap<Coord, Vec<Coord>> = HashMap::new();
        for swarm_index in 0..self.swarms.len() {
            let swarm = self.swarms[swarm_index].clone();
            let base = *ctx.rng.choice(&self.config.base_color);
            let swarm_gradient = Gradient::with_steps(&[base, self.config.flash_color], 7, false)
                .map_err(EngineError::Other)?;
            let swarm_gradient_mirror: Vec<Color> = swarm_gradient
                .spectrum
                .iter()
                .copied()
                .chain(flash_list.iter().copied())
                .chain(swarm_gradient.spectrum.iter().rev().copied())
                .collect();
            // dict[Coord, list[Coord]] — insertion-ordered with key-overwrite-in-place
            let mut swarm_area_coordinate_map: Vec<(Coord, Vec<Coord>)> = Vec::new();
            let swarm_spawn = ctx.terminal.canvas.random_coord(&mut ctx.rng, true, false);
            let mut swarm_areas: Vec<Coord> = Vec::new();
            let swarm_area_count = ctx.rng.randint(
                self.config.swarm_area_count_range.0,
                self.config.swarm_area_count_range.1,
            );
            // create areas where characters will swarm
            let mut last_focus_coord = swarm_spawn;
            let radius = std::cmp::max(floor_div(std::cmp::min(canvas_right, canvas_top), 2), 1);
            while (swarm_areas.len() as i64) < swarm_area_count {
                // Upstream find_coords_on_circle is lru_cached and swarm shuffles the
                // RETURNED LIST IN PLACE, mutating the cache: a later call with the
                // same focus coord returns the previously shuffled list, not the
                // freshly computed one. Reproduce that observable quirk with an
                // effect-local cache whose entries persist the shuffle mutation
                // (the only in-process mutator of this cache is this loop).
                let cached = circle_cache.entry(last_focus_coord).or_insert_with(|| {
                    geometry::find_coords_on_circle(last_focus_coord, radius, 0, true)
                });
                ctx.rng.shuffle(cached);
                let potential_focus_coords = cached.clone();
                let mut next_focus_coord: Option<Coord> = None;
                for coord in &potential_focus_coords {
                    if ctx.terminal.canvas.coord_is_in_canvas(*coord) {
                        next_focus_coord = Some(*coord);
                        break;
                    }
                }
                let next_focus_coord = match next_focus_coord {
                    Some(coord) => coord,
                    None => ctx.terminal.canvas.random_coord(&mut ctx.rng, false, false),
                };
                swarm_areas.push(next_focus_coord);
                let area_coords = geometry::find_coords_in_circle(
                    last_focus_coord,
                    std::cmp::max(floor_div(std::cmp::min(canvas_right, canvas_top), 6), 1) * 2,
                );
                if let Some(entry) = swarm_area_coordinate_map
                    .iter_mut()
                    .find(|(coord, _)| *coord == last_focus_coord)
                {
                    entry.1 = area_coords;
                } else {
                    swarm_area_coordinate_map.push((last_focus_coord, area_coords));
                }
                last_focus_coord = next_focus_coord;
            }

            // assign characters waypoints for swarm areas and inner waypoints within the swarm areas
            for &id in &swarm {
                self.setup_swarm_character(
                    ctx,
                    id,
                    swarm_spawn,
                    &swarm_gradient_mirror,
                    &swarm_area_coordinate_map,
                    dynamic,
                    &dynamic_clear_color,
                )?;
            }
        }
        self.call_next = true;
        self.active_swarm_area = "0_swarm_area".to_string();
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        self.step_scene(ctx)
    }
}
