// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{EngineCtx, Swarm, SwarmConfig, emit_frame, first_char_digit};

impl Swarm {
    pub(super) fn step_scene(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.swarms.is_empty() || !ctx.active_characters.is_empty() {
            if !self.swarms.is_empty() && self.call_next {
                self.call_next = false;
                self.current_swarm = self.swarms.pop()?;
                // Reuse the existing allocation: the steady-state path must
                // not allocate per frame.
                self.active_swarm_area.clear();
                self.active_swarm_area.push_str("0_swarm_area");
                for i in 0..self.current_swarm.len() {
                    let id = self.current_swarm[i];
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
                // Find the member that reached the next swarm area. The
                // borrow of the arena ends before the mutation below, so the
                // per-member path no longer has to be cloned every frame.
                let mut next_area: Option<(usize, String)> = None;
                {
                    let arena = &ctx.terminal.arena;
                    for i in 0..self.current_swarm.len() {
                        let id = self.current_swarm[i];
                        let path_id = &arena[id.0 as usize].motion.active_path;
                        if let Some(path_id) = path_id
                            && path_id.as_ref() != self.active_swarm_area
                            && path_id.contains("swarm_area")
                            && first_char_digit(path_id) > first_char_digit(&self.active_swarm_area)
                        {
                            next_area = Some((i, path_id.to_string()));
                            break;
                        }
                    }
                }
                if let Some((idx, area)) = next_area {
                    let id = self.current_swarm[idx];
                    self.active_swarm_area.clone_from(&area);
                    for i in 0..self.current_swarm.len() {
                        let other = self.current_swarm[i];
                        if other != id && ctx.rng.random() < self.config.swarm_coordination {
                            ctx.activate_path(self, other, &area);
                        }
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
