// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Blackhole {
    /// BlackholeIterator.prepare_blackhole.
    pub(super) fn prepare_blackhole(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let star_symbols = ["*", "'", "`", "¤", "•", "°", "·"];
        let starfield_colors = Gradient::with_steps(
            &[
                Color::from_hex("#4a4a4d").must_val(),
                Color::from_hex("#ffffff").must_val(),
            ],
            6,
            false,
        )
        .map_err(EngineError::Other)?
        .spectrum;
        // gradient_map: dict keyed by starfield color, indexed here by spectrum position
        let mut gradient_map: Vec<Gradient> = Vec::new();
        for color in &starfield_colors {
            gradient_map.push(
                Gradient::with_steps(
                    &[color.clone(), Color::from_hex("#000000").must_val()],
                    10,
                    false,
                )
                .map_err(EngineError::Other)?,
            );
        }
        let mut available_chars: Vec<CharId> = ctx.terminal.input_characters.clone();
        while (self.blackhole_chars.len() as i64) < self.blackhole_radius * 3
            && !available_chars.is_empty()
        {
            let index = ctx.rng.randrange(0, available_chars.len() as i64) as usize;
            self.blackhole_chars.push(available_chars.remove(index));
        }
        let black_hole_ring_positions = geometry::find_coords_on_circle(
            ctx.terminal.canvas.center,
            self.blackhole_radius,
            self.blackhole_chars.len() as i64,
            true,
        );
        for (position_index, &id) in self.blackhole_chars.iter().enumerate() {
            let starting_pos = black_hole_ring_positions[position_index];
            let blackhole_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let path_id = ch
                    .motion
                    .new_path(0.7, Some(Easing::InOutSine), None, 0, false, "blackhole")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .must_val()
                    .new_waypoint(starting_pos, None, "")
                    .map_err(EngineError::Other)?;
                path_id
            };
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let uses_pre = ch.uses_input_preexisting_colors;
                let blackhole_scn =
                    ch.animation
                        .new_scene(false, None, None, "blackhole", uses_pre);
                ch.animation
                    .scenes
                    .get_mut(&blackhole_scn)
                    .must_val()
                    .add_frame(
                        "*",
                        1,
                        VisualParams {
                            colors: Some(ColorPair::new(
                                Some(self.config.blackhole_color.clone()),
                                None,
                            )),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            ctx.register_event(
                id,
                Event::PathActivated,
                CallerKey::Path(blackhole_path),
                EventAction::SetLayer(1),
            )
            .map_err(EngineError::Other)?;
            // make rotation waypoints
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let rotation_path = ch
                    .motion
                    .new_path(0.45, None, None, 0, true, "blackhole_rotation")
                    .map_err(EngineError::Other)?;
                let rotated: Vec<_> = black_hole_ring_positions[position_index..]
                    .iter()
                    .chain(black_hole_ring_positions[..position_index].iter())
                    .copied()
                    .collect();
                for coord in rotated {
                    let path = ch.motion.paths.get_mut(&rotation_path).must_val();
                    let waypoint_id = path.waypoints.len().to_string();
                    path.new_waypoint(coord, None, &waypoint_id)
                        .map_err(EngineError::Other)?;
                }
            }
        }
        let blackhole_set: HashSet<CharId> = self.blackhole_chars.iter().copied().collect();
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        let canvas_center = ctx.terminal.canvas.center;
        for &id in &characters {
            ctx.terminal.set_character_visibility(id, true);
            let star_symbol = *ctx.rng.choice(&star_symbols);
            let star_color_index = {
                // random.choice(starfield_colors)
                let _ = &starfield_colors;
                ctx.rng.choice_index(starfield_colors.len())
            };
            let star_color = starfield_colors[star_color_index].clone();
            let starting_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let uses_pre = ch.uses_input_preexisting_colors;
                let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                ch.animation
                    .scenes
                    .get_mut(&scene_id)
                    .must_val()
                    .add_frame(
                        star_symbol,
                        1,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(star_color.clone()), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                scene_id
            };
            ctx.activate_scene(self, id, &starting_scn);
            if !blackhole_set.contains(&id) {
                let starfield_coord = ctx.terminal.canvas.random_coord(&mut ctx.rng, false, false);
                let speed = ctx.rng.uniform(0.17, 0.30);
                let singularity_path = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.motion.set_coordinate(starfield_coord);
                    let path_id = ch
                        .motion
                        .new_path(speed, Some(Easing::InExpo), None, 0, false, "singularity")
                        .map_err(EngineError::Other)?;
                    ch.motion
                        .paths
                        .get_mut(&path_id)
                        .must_val()
                        .new_waypoint(canvas_center, None, "")
                        .map_err(EngineError::Other)?;
                    path_id
                };
                let consumed_scn = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let uses_pre = ch.uses_input_preexisting_colors;
                    let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                    let scene = ch.animation.scenes.get_mut(&scene_id).must_val();
                    for color in &gradient_map[star_color_index].spectrum {
                        scene
                            .add_frame(
                                star_symbol,
                                1,
                                VisualParams {
                                    colors: Some(ColorPair::new(Some(color.clone()), None)),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                    scene
                        .add_frame(" ", 1, VisualParams::default())
                        .map_err(EngineError::Other)?;
                    scene.sync = Some(SyncMetric::Distance);
                    scene_id
                };
                ctx.register_event(
                    id,
                    Event::PathActivated,
                    CallerKey::Path(singularity_path.clone()),
                    EventAction::SetLayer(2),
                )
                .map_err(EngineError::Other)?;
                ctx.register_event(
                    id,
                    Event::PathActivated,
                    CallerKey::Path(singularity_path),
                    EventAction::ActivateScene(consumed_scn),
                )
                .map_err(EngineError::Other)?;
                self.awaiting_consumption_chars.push(id);
            }
        }
        ctx.rng.shuffle(&mut self.awaiting_consumption_chars);
        Ok(())
    }

    /// BlackholeIterator.rotate_blackhole.
    pub(super) fn rotate_blackhole(&mut self, ctx: &mut EngineCtx) {
        for &id in &self.blackhole_chars.clone() {
            ctx.activate_path(self, id, "blackhole_rotation");
            ctx.active_characters.insert(id);
        }
    }
}
