// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Bubbles {
    pub fn new(config: BubblesConfig) -> Self {
        let rainbow_stops = [
            Color::from_hex("e81416").must_val(), // red
            Color::from_hex("ffa500").must_val(), // orange
            Color::from_hex("faeb36").must_val(), // yellow
            Color::from_hex("79c314").must_val(), // green
            Color::from_hex("487de7").must_val(), // blue
            Color::from_hex("4b369d").must_val(), // indigo
            Color::from_hex("70369d").must_val(), // violet
        ];
        let rainbow_gradient = Gradient::with_steps(&rainbow_stops, 5, false).must_val();
        Bubbles {
            config,
            bubbles: Vec::new(),
            animating_bubbles: Vec::new(),
            rainbow_gradient,
            character_final_color_map: HashMap::new(),
            steps_since_last_bubble: 0,
        }
    }

    /// Bubble.set_character_coordinates.
    pub(super) fn bubble_set_character_coordinates(
        &mut self,
        ctx: &mut EngineCtx,
        bubble: &mut Bubble,
    ) {
        let anchor_coord = ctx.terminal.arena[bubble.anchor_char.0 as usize]
            .motion
            .current_coord;
        let points = geometry::find_coords_on_circle(
            anchor_coord,
            bubble.radius,
            bubble.characters.len() as i64,
            false,
        );
        for (i, &id) in bubble.characters.iter().enumerate() {
            let point = points[i];
            ctx.terminal.arena[id.0 as usize]
                .motion
                .set_coordinate(point);
            if point.row == bubble.lowest_row {
                bubble.landed = true;
            }
        }
        if self.config.pop_condition == PopCondition::Anywhere && ctx.rng.random() < 0.002 {
            bubble.landed = true;
        }
    }

    /// Bubble.__init__ (+ make_waypoints + make_gradients).
    pub(super) fn make_bubble(
        &mut self,
        ctx: &mut EngineCtx,
        origin: Coord,
        characters: Vec<CharId>,
    ) -> Result<Bubble, EngineError> {
        let radius = std::cmp::max(characters.len() as i64 / 5, 1);
        let anchor_char = ctx.terminal.add_character(" ", origin);
        let lowest_row = if self.config.pop_condition == PopCondition::Row {
            characters
                .iter()
                .map(|&id| ctx.terminal.arena[id.0 as usize].input_coord.row)
                .min()
                .must_val()
        } else {
            ctx.terminal.canvas.bottom
        };
        let mut bubble = Bubble {
            characters,
            radius,
            anchor_char,
            lowest_row,
            landed: false,
        };
        self.bubble_set_character_coordinates(ctx, &mut bubble);
        bubble.landed = false;
        // make_waypoints
        let waypoint_column = ctx
            .rng
            .randint(ctx.terminal.canvas.left, ctx.terminal.canvas.right);
        let floor_path = {
            let ch = &mut ctx.terminal.arena[bubble.anchor_char.0 as usize];
            let path_id = ch
                .motion
                .new_path(self.config.bubble_speed, None, None, 0, false, "")
                .map_err(EngineError::Other)?;
            ch.motion
                .paths
                .get_mut(&path_id)
                .must_val()
                .new_waypoint(Coord::new(waypoint_column, bubble.lowest_row), None, "")
                .map_err(EngineError::Other)?;
            path_id
        };
        ctx.activate_path(self, bubble.anchor_char, &floor_path);
        // make_gradients
        if self.config.rainbow {
            let mut rainbow_gradient: Vec<Color> = self.rainbow_gradient.spectrum.clone();
            let mut gradient_offset: usize = 0;
            for &id in &bubble.characters {
                let (input_symbol, uses_pre) = {
                    let ch = &ctx.terminal.arena[id.0 as usize];
                    (ch.input_symbol.clone(), ch.uses_input_preexisting_colors)
                };
                let sheen_scene = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                    let scene = ch.animation.scenes.get_mut(&scene_id).must_val();
                    for step in &rainbow_gradient {
                        scene
                            .add_frame(
                                &input_symbol,
                                4,
                                VisualParams {
                                    colors: Some(ColorPair::new(Some(step.clone()), None)),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                    scene_id
                };
                gradient_offset += 2;
                gradient_offset %= rainbow_gradient.len();
                let mut rotated = rainbow_gradient[gradient_offset..].to_vec();
                rotated.extend_from_slice(&rainbow_gradient[..gradient_offset]);
                rainbow_gradient = rotated;
                ctx.activate_scene(self, id, &sheen_scene);
                let active_scene = ctx.terminal.arena[id.0 as usize]
                    .animation
                    .active_scene
                    .clone();
                if let Some(scene_id) = active_scene {
                    ctx.terminal.arena[id.0 as usize]
                        .animation
                        .scenes
                        .get_mut(&scene_id)
                        .must_val()
                        .is_looping = true;
                }
            }
        } else {
            let bubble_color = ctx.rng.choice(&self.config.bubble_colors).clone();
            for &id in &bubble.characters {
                let (input_symbol, uses_pre) = {
                    let ch = &ctx.terminal.arena[id.0 as usize];
                    (ch.input_symbol.clone(), ch.uses_input_preexisting_colors)
                };
                let sheen_scene = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                    ch.animation
                        .scenes
                        .get_mut(&scene_id)
                        .must_val()
                        .add_frame(
                            &input_symbol,
                            1,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(bubble_color.clone()), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                    scene_id
                };
                ctx.activate_scene(self, id, &sheen_scene);
            }
        }
        Ok(bubble)
    }

    /// Bubble.pop.
    pub(super) fn bubble_pop(&mut self, ctx: &mut EngineCtx, bubble: &Bubble) {
        let anchor_coord = ctx.terminal.arena[bubble.anchor_char.0 as usize]
            .motion
            .current_coord;
        let points = geometry::find_coords_on_circle(
            anchor_coord,
            bubble.radius + 3,
            bubble.characters.len() as i64,
            true,
        );
        // zip(characters, points) — truncates at the shorter sequence
        for (&id, &point) in bubble.characters.iter().zip(points.iter()) {
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let pop_out_path = ch
                    .motion
                    .new_path(0.3, Some(Easing::OutExpo), None, 0, false, "pop_out")
                    .must_val();
                ch.motion
                    .paths
                    .get_mut(&pop_out_path)
                    .must_val()
                    .new_waypoint(point, None, "")
                    .must_val();
            }
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path("pop_out".to_string()),
                EventAction::ActivatePath("final".to_string()),
            )
            .must_val();
        }
        for &id in &bubble.characters {
            ctx.activate_scene(self, id, "pop_1");
            ctx.activate_path(self, id, "pop_out");
        }
    }
}
impl Bubbles {
    /// Bubble.move.
    pub(super) fn bubble_move(&mut self, ctx: &mut EngineCtx, bubble: &mut Bubble) {
        ctx.motion_move(self, bubble.anchor_char);
        self.bubble_set_character_coordinates(ctx, bubble);
        for i in 0..bubble.characters.len() {
            let id = bubble.characters[i];
            ctx.step_animation(self, id);
        }
    }
}
impl EffectHooks for Bubbles {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
