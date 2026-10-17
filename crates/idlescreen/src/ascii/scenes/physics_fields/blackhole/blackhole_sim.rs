// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    Blackhole, CallerKey, ColorPair, Easing, EngineCtx, EngineError, Event, EventAction,
    VisualParams, geometry,
};

impl Blackhole {
    /// BlackholeIterator.collapse_blackhole.
    pub(super) fn collapse_blackhole(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let mut black_hole_ring_positions = geometry::find_coords_on_circle(
            ctx.terminal.canvas.center,
            self.blackhole_radius + 3,
            self.blackhole_chars.len() as i64,
            true,
        );
        let unstable_symbols = ["◦", "◎", "◉", "●", "◉", "◎", "◦"];
        let mut point_char_made = false;
        let canvas_center = ctx.terminal.canvas.center;
        for i in 0..self.blackhole_chars.len() {
            let id = self.blackhole_chars[i];
            let next_pos = black_hole_ring_positions.remove(0);
            let (expand_path, collapse_path) = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let expand_path = ch
                    .motion
                    .new_path(0.2, Some(Easing::InExpo), None, 0, false, "")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&expand_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(next_pos, None, "")
                    .map_err(EngineError::Other)?;
                let collapse_path = ch
                    .motion
                    .new_path(0.3, Some(Easing::InExpo), None, 0, false, "")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&collapse_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(canvas_center, None, "")
                    .map_err(EngineError::Other)?;
                (expand_path, collapse_path)
            };
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(expand_path.clone()),
                EventAction::ActivatePath(collapse_path.clone()),
            )
            .map_err(EngineError::Other)?;
            if !point_char_made {
                let point_scn = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let uses_pre = ch.uses_input_preexisting_colors;
                    ch.animation.new_scene(false, None, None, "", uses_pre)
                };
                for _ in 0..3 {
                    for symbol in unstable_symbols {
                        let color = *ctx.rng.choice(&self.config.star_colors);
                        let ch = &mut ctx.terminal.arena[id.0 as usize];
                        ch.animation
                            .scenes
                            .get_mut(&point_scn)
                            .ok_or_else(|| EngineError::Other(String::new()))?
                            .add_frame(
                                symbol,
                                3,
                                VisualParams {
                                    colors: Some(ColorPair::new(Some(color), None)),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                }
                ctx.register_event(
                    id,
                    Event::PathComplete,
                    CallerKey::Path(collapse_path.clone()),
                    EventAction::ActivateScene(point_scn),
                )
                .map_err(EngineError::Other)?;
                ctx.register_event(
                    id,
                    Event::PathComplete,
                    CallerKey::Path(collapse_path),
                    EventAction::SetLayer(3),
                )
                .map_err(EngineError::Other)?;
                point_char_made = true;
            }

            ctx.activate_path(self, id, &expand_path);
            ctx.active_characters.insert(id);
        }
        Ok(())
    }
}
