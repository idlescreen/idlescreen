// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{EngineCtx, Swarm, SwarmConfig, emit_frame, first_char_digit};

impl Swarm {
    pub(super) fn step_scene(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.swarms.is_empty() || !ctx.active_characters.is_empty() {
            if !self.swarms.is_empty() && self.call_next {
                self.call_next = false;
                self.current_swarm = self.swarms.pop()?;
                self.active_swarm_area = "0_swarm_area".to_string();
                for &id in &self.current_swarm.clone() {
                    ctx.activate_path(self, id, "0_swarm_area");
                    ctx.terminal.set_character_visibility(id, true);
                    ctx.active_characters.insert(id);
                }
            }
            if ctx.active_characters.len() < self.current_swarm.len() {
                // some of the characters have landed
                self.call_next = true;
            }
            if !self.current_swarm.is_empty() {
                for i in 0..self.current_swarm.len() {
                    let id = self.current_swarm[i];
                    let active_path_id =
                        ctx.terminal.arena[id.0 as usize].motion.active_path.clone();
                    if let Some(path_id) = active_path_id
                        && path_id.as_ref() != self.active_swarm_area
                        && path_id.contains("swarm_area")
                        && first_char_digit(&path_id) > first_char_digit(&self.active_swarm_area)
                    {
                        self.active_swarm_area = path_id.to_string();
                        for &other in &self.current_swarm.clone() {
                            if other != id && ctx.rng.random() < self.config.swarm_coordination {
                                let area = self.active_swarm_area.clone();
                                ctx.activate_path(self, other, &area);
                            }
                        }
                        break;
                    }
                }
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Swarm {
    fn default() -> Self {
        Self::new(SwarmConfig::default())
    }
}
