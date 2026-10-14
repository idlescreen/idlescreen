// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for Rings {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.build_scene(ctx)
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.phase == Phase::Complete {
            return None;
        }
        match self.phase {
            Phase::Start => {
                if self.initial_phase_time_remaining == 0 {
                    self.phase = Phase::Disperse;
                } else {
                    self.initial_phase_time_remaining -= 1;
                }
            }
            Phase::Disperse => {
                if !self.initial_disperse_complete {
                    self.initial_disperse_complete = true;
                    let mut rings = std::mem::take(&mut self.rings);
                    for ring in &mut rings {
                        let characters = ring.characters.clone();
                        for id in characters {
                            let ring_start_coord = ctx.terminal.arena[id.0 as usize]
                                .motion
                                .paths
                                .get("0")
                                .must_val()
                                .waypoints[0]
                                .coord;
                            let disperse_path = ring
                                .make_disperse_waypoints(ctx, id, ring_start_coord)
                                .must_val();
                            let initial_path = {
                                let ch = &mut ctx.terminal.arena[id.0 as usize];
                                let disperse_first_coord =
                                    ch.motion.paths.get(&disperse_path).must_val().waypoints[0]
                                        .coord;
                                let path_id = ch
                                    .motion
                                    .new_path(0.3, Some(Easing::OutCubic), None, 0, false, "")
                                    .must_val();
                                ch.motion
                                    .paths
                                    .get_mut(&path_id)
                                    .must_val()
                                    .new_waypoint(disperse_first_coord, None, "")
                                    .must_val();
                                path_id
                            };
                            ctx.register_event(
                                id,
                                Event::PathComplete,
                                CallerKey::Path(initial_path.clone()),
                                EventAction::ActivatePath(disperse_path),
                            )
                            .must_val();
                            ctx.activate_scene(self, id, "disperse");
                            ctx.activate_path(self, id, &initial_path);
                            ctx.active_characters.insert(id);
                        }
                    }
                    self.rings = rings;

                    let non_ring_chars = self.non_ring_chars.clone();
                    for id in non_ring_chars {
                        ctx.activate_path(self, id, "external");
                        ctx.active_characters.insert(id);
                    }
                } else if self.disperse_time_remaining == 0 {
                    self.phase = Phase::Spin;
                    self.cycles_remaining -= 1;
                    self.spin_time_remaining = self.config.spin_duration;
                    let mut rings = std::mem::take(&mut self.rings);
                    for ring in &mut rings {
                        self.ring_spin(ctx, ring).must_val();
                    }
                    self.rings = rings;
                } else {
                    self.disperse_time_remaining -= 1;
                }
            }
            Phase::Spin => {
                if self.spin_time_remaining == 0 {
                    if self.cycles_remaining == 0 {
                        self.phase = Phase::Final;
                        let characters = {
                            let filter = CharacterFilter::default();
                            ctx.terminal.get_characters(
                                &mut ctx.rng,
                                filter,
                                CharacterSort::TopToBottomLeftToRight,
                            )
                        };
                        for id in characters {
                            ctx.terminal.set_character_visibility(id, true);
                            ctx.activate_path(self, id, "home");
                            ctx.active_characters.insert(id);
                            if ctx.terminal.arena[id.0 as usize]
                                .motion
                                .paths
                                .contains_key("external")
                            {
                                continue;
                            }
                            ctx.activate_scene(self, id, "disperse");
                        }
                    } else {
                        self.disperse_time_remaining = self.config.disperse_duration;
                        let mut rings = std::mem::take(&mut self.rings);
                        for ring in &mut rings {
                            self.ring_disperse(ctx, ring).must_val();
                        }
                        self.rings = rings;
                        self.phase = Phase::Disperse;
                    }
                } else {
                    self.spin_time_remaining -= 1;
                }
            }
            Phase::Final => {
                if ctx.active_characters.is_empty() {
                    self.phase = Phase::Complete;
                }
            }
            Phase::Complete => unreachable!(),
        }
        ctx.update(self);
        Some(emit_frame(ctx))
    }
}
impl Default for RingsConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Rings {
    fn default() -> Self {
        Self::new(RingsConfig::default())
    }
}
