// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for Blackhole {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // BlackholeIterator.__init__
        self.blackhole_radius = std::cmp::max(
            std::cmp::min(
                round_half_even(ctx.terminal.canvas.width as f64 * 0.3),
                round_half_even(ctx.terminal.canvas.height as f64 * 0.20),
            ),
            3,
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
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for &id in &characters {
            let input_coord = ctx.terminal.arena[id.0 as usize].input_coord;
            self.character_final_color_map.insert(
                id,
                final_gradient_mapping.get(&input_coord).must_val().clone(),
            );
        }
        self.prepare_blackhole(ctx)?;
        self.formation_delay = std::cmp::max(floor_div(100, self.blackhole_chars.len() as i64), 3);
        self.f_delay = self.formation_delay;
        self.phase = Phase::Forming;
        self.awaiting_blackhole_chars = self.blackhole_chars.clone();
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !ctx.active_characters.is_empty() || self.phase != Phase::Complete {
            match self.phase {
                Phase::Forming => {
                    if !self.awaiting_blackhole_chars.is_empty() {
                        if self.f_delay == 0 {
                            let next_char = self.awaiting_blackhole_chars.remove(0);
                            ctx.activate_path(self, next_char, "blackhole");
                            ctx.activate_scene(self, next_char, "blackhole");
                            ctx.active_characters.insert(next_char);
                            self.f_delay = self.formation_delay;
                        } else {
                            self.f_delay -= 1;
                        }
                    } else if ctx.active_characters.is_empty() {
                        self.rotate_blackhole(ctx);
                        self.phase = Phase::Consuming;
                    }
                }
                Phase::Consuming => {
                    if !self.awaiting_consumption_chars.is_empty() {
                        for &id in &self.awaiting_consumption_chars.clone() {
                            ctx.activate_path(self, id, "singularity");
                            ctx.active_characters.insert(id);
                        }
                        self.awaiting_consumption_chars.clear();
                    } else {
                        let blackhole_set: HashSet<CharId> =
                            self.blackhole_chars.iter().copied().collect();
                        if ctx
                            .active_characters
                            .iter()
                            .all(|id| blackhole_set.contains(&id))
                        {
                            self.phase = Phase::Collapsing;
                        }
                    }
                }
                Phase::Collapsing => {
                    self.collapse_blackhole(ctx).must_val();
                    self.phase = Phase::Exploding;
                }
                Phase::Exploding => {
                    if self.blackhole_chars.iter().all(|&id| {
                        let ch = &ctx.terminal.arena[id.0 as usize];
                        ch.motion.active_path.is_none() && ch.animation.active_scene.is_none()
                    }) {
                        self.explode_singularity(ctx).must_val();
                        self.phase = Phase::Complete;
                    }
                }
                Phase::Complete => {}
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}
impl Default for BlackholeConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Blackhole {
    fn default() -> Self {
        Self::new(BlackholeConfig::default())
    }
}
