// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CallerKey, CharId, Coord, EngineCtx, EngineError, Event, EventAction, ExistingColorHandling,
    Gradient, HashMap, Phase, Ring, Rings, RingsConfig, VisualParams,
};

impl Rings {
    pub fn new(config: RingsConfig) -> Self {
        let spin_time_remaining = config.spin_duration;
        let disperse_time_remaining = config.disperse_duration;
        let cycles_remaining = config.spin_disperse_cycles;
        Rings {
            config,
            rings: Vec::new(),
            non_ring_chars: Vec::new(),
            character_final_color_map: HashMap::new(),
            phase: Phase::Start,
            initial_disperse_complete: false,
            spin_time_remaining,
            disperse_time_remaining,
            cycles_remaining,
            initial_phase_time_remaining: 100,
        }
    }

    /// Ring.add_character (needs the effect-level color map, so lives here).
    pub(super) fn ring_add_character(
        &mut self,
        ctx: &mut EngineCtx,
        ring: &mut Ring,
        id: CharId,
        clockwise: bool,
    ) -> Result<(), EngineError> {
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let (input_symbol, uses_pre) = {
            let ch = &ctx.terminal.arena[id.0 as usize];
            (ch.input_symbol.clone(), ch.uses_input_preexisting_colors)
        };
        // make gradient scene
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let gradient_scn = ch
                .animation
                .new_scene(false, None, None, "gradient", uses_pre);
            let scene = ch
                .animation
                .scenes
                .get_mut(&gradient_scn)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            if dynamic {
                let colors = *self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                scene
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            } else {
                let final_fg_color = self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .fg_color
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let char_gradient =
                    Gradient::with_steps(&[final_fg_color, ring.ring_color], 8, false)
                        .map_err(EngineError::Other)?;
                scene
                    .apply_gradient_to_symbols(
                        std::slice::from_ref(&input_symbol),
                        3,
                        Some(&char_gradient),
                        None,
                    )
                    .map_err(EngineError::Other)?;
            }
        }

        // make rotation waypoints
        let mut ring_paths: Vec<String> = Vec::new();
        let character_starting_index = ring.characters.len();
        let coords = if clockwise {
            &ring.clockwise_coords
        } else {
            &ring.counter_clockwise_coords
        };
        let rotated: Vec<Coord> = coords[character_starting_index..]
            .iter()
            .chain(coords[..character_starting_index].iter())
            .copied()
            .collect();
        for coord in rotated {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let path_id = ch
                .motion
                .new_path(
                    ring.rotation_speed,
                    None,
                    None,
                    0,
                    false,
                    &ring_paths.len().to_string(),
                )
                .map_err(EngineError::Other)?;
            let path = ch
                .motion
                .paths
                .get_mut(&path_id)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            let waypoint_id = path.waypoints.len().to_string();
            path.new_waypoint(coord, None, &waypoint_id)
                .map_err(EngineError::Other)?;
            ring_paths.push(path_id);
        }
        ring.character_last_ring_path
            .insert(id, ring_paths[0].clone());

        // make disperse scene
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let disperse_scn = ch
                .animation
                .new_scene(false, None, None, "disperse", uses_pre);
            let scene = ch
                .animation
                .scenes
                .get_mut(&disperse_scn)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            if dynamic {
                let colors = *self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                scene
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            } else {
                let final_fg_color = self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .fg_color
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let disperse_gradient =
                    Gradient::with_steps(&[ring.ring_color, final_fg_color], 8, false)
                        .map_err(EngineError::Other)?;
                scene
                    .apply_gradient_to_symbols(
                        std::slice::from_ref(&input_symbol),
                        10,
                        Some(&disperse_gradient),
                        None,
                    )
                    .map_err(EngineError::Other)?;
            }
        }
        ctx.chain_paths(id, &ring_paths, true)
            .map_err(EngineError::Other)?;
        ring.characters.push(id);
        Ok(())
    }

    /// Ring.disperse.
    pub(super) fn ring_disperse(
        &mut self,
        ctx: &mut EngineCtx,
        ring: &mut Ring,
    ) -> Result<(), EngineError> {
        let characters = ring.characters.clone();
        for id in characters {
            let (active_path, current_coord) = {
                let motion = &ctx.terminal.arena[id.0 as usize].motion;
                (motion.active_path.clone(), motion.current_coord)
            };
            let last = match active_path {
                Some(path_id) => path_id.to_string(),
                None => "0".to_string(),
            };
            ring.character_last_ring_path.insert(id, last);
            let disperse_path = ring.make_disperse_waypoints(ctx, id, current_coord)?;
            ctx.activate_path(self, id, &disperse_path);
            ctx.activate_scene(self, id, "disperse");
        }
        Ok(())
    }

    /// Ring.spin.
    pub(super) fn ring_spin(
        &mut self,
        ctx: &mut EngineCtx,
        ring: &mut Ring,
    ) -> Result<(), EngineError> {
        let characters = ring.characters.clone();
        for id in characters {
            let last_ring_path = ring
                .character_last_ring_path
                .get(&id)
                .ok_or_else(|| EngineError::Other(String::new()))?
                .clone();
            let condense_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let first_waypoint_coord = ch
                    .motion
                    .paths
                    .get(&last_ring_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .waypoints[0]
                    .coord;
                let path_id = ch
                    .motion
                    .new_path(0.1, None, None, 0, false, "")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(first_waypoint_coord, None, "")
                    .map_err(EngineError::Other)?;
                path_id
            };
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(condense_path.clone()),
                EventAction::ActivatePath(last_ring_path),
            )
            .map_err(EngineError::Other)?;
            ctx.activate_path(self, id, &condense_path);
            ctx.activate_scene(self, id, "gradient");
        }
        Ok(())
    }
}
