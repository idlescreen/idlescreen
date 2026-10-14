// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for Beams {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.build_scene(ctx)
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.phase != Phase::Complete || !ctx.active_characters.is_empty() {
            match self.phase {
                Phase::Beams => {
                    if self.delay == 0 {
                        if !self.pending_groups.is_empty() {
                            for _ in 0..ctx.rng.randint(1, 5) {
                                if !self.pending_groups.is_empty() {
                                    self.active_groups.push(self.pending_groups.remove(0));
                                }
                            }
                        }
                        self.delay = self.config.beam_delay;
                    } else {
                        self.delay -= 1;
                    }
                    let mut active_groups = std::mem::take(&mut self.active_groups);
                    for group in &mut active_groups {
                        group.next_character_counter += group.speed;
                        // int() truncation
                        let count = group.next_character_counter as i64;
                        if count > 1 {
                            for _ in 0..count {
                                if !group.characters.is_empty() {
                                    if let Some(next_char) = self.get_next_character(ctx, group) {
                                        ctx.active_characters.insert(next_char);
                                    }
                                }
                            }
                        }
                    }
                    active_groups.retain(|group| !group.characters.is_empty());
                    self.active_groups = active_groups;
                    if self.pending_groups.is_empty()
                        && self.active_groups.is_empty()
                        && ctx.active_characters.is_empty()
                    {
                        self.phase = Phase::FinalWipe;
                    }
                }
                Phase::FinalWipe => {
                    if !self.final_wipe_groups.is_empty() {
                        for _ in 0..self.config.final_wipe_speed {
                            if self.final_wipe_groups.is_empty() {
                                break;
                            }
                            let next_group = self.final_wipe_groups.remove(0);
                            for id in next_group {
                                ctx.activate_scene(self, id, "brighten");
                                ctx.terminal.set_character_visibility(id, true);
                                ctx.active_characters.insert(id);
                            }
                        }
                    } else {
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
impl Default for BeamsConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Beams {
    fn default() -> Self {
        Self::new(BeamsConfig::default())
    }
}
