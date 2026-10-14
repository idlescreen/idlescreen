// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Crumble {
    pub(super) fn setup_crumble_stage(
        &mut self,
        ctx: &mut EngineCtx,
        id: CharId,
        input_symbol: &str,
        input_coord: Coord,
        uses_pre: bool,
        dynamic_uncolored: bool,
        fall_path: String,
        (dust_fg_color, dust_bg_color): (Option<Color>, Option<Color>),
        (strengthen_flash_fg_gradient, strengthen_flash_bg_gradient): (
            Option<Gradient>,
            Option<Gradient>,
        ),
        (strengthen_fg_gradient, strengthen_bg_gradient): (Option<Gradient>, Option<Gradient>),
    ) -> Result<(), EngineError> {
        let canvas_top = ctx.terminal.canvas.top;
        let canvas_center_column = ctx.terminal.canvas.center_column;
        let canvas_center_row = ctx.terminal.canvas.center_row;
        let input_symbol = input_symbol.to_string();
        {
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let top_path = ch
                    .motion
                    .new_path(1.0, Some(Easing::OutQuint), None, 0, false, "top")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&top_path)
                    .must_val()
                    .new_waypoint(
                        Coord::new(input_coord.column, canvas_top),
                        Some(vec![Coord::new(canvas_center_column, canvas_center_row)]),
                        "",
                    )
                    .map_err(EngineError::Other)?;
            }
            // set up reset stage
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let input_path = ch
                    .motion
                    .new_path(1.6, None, None, 0, false, "input")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&input_path)
                    .must_val()
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
            }
            let strengthen_flash_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                ch.animation
                    .scenes
                    .get_mut(&scene_id)
                    .must_val()
                    .apply_gradient_to_symbols(
                        &[input_symbol.clone()],
                        4,
                        strengthen_flash_fg_gradient.as_ref(),
                        strengthen_flash_bg_gradient.as_ref(),
                    )
                    .map_err(EngineError::Other)?;
                scene_id
            };
            let strengthen_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                let scene = ch.animation.scenes.get_mut(&scene_id).must_val();
                if dynamic_uncolored {
                    scene
                        .add_frame(
                            &input_symbol,
                            4,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            4,
                            strengthen_fg_gradient.as_ref(),
                            strengthen_bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                }
                scene_id
            };
            let dust_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .new_scene(false, Some(SyncMetric::Distance), None, "", uses_pre)
            };
            let dust_symbols = ["*", ".", ","];
            for _ in 0..5 {
                let symbol = *ctx.rng.choice(&dust_symbols);
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&dust_scn)
                    .must_val()
                    .add_frame(
                        symbol,
                        1,
                        VisualParams {
                            colors: Some(ColorPair::new(
                                dust_fg_color.clone(),
                                dust_bg_color.clone(),
                            )),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }

            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene("weaken".to_string()),
                EventAction::ActivatePath(fall_path),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene("weaken".to_string()),
                EventAction::SetLayer(1),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene("weaken".to_string()),
                EventAction::ActivateScene(dust_scn),
            )
            .map_err(EngineError::Other)?;

            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path("input".to_string()),
                EventAction::ActivateScene(strengthen_flash_scn.clone()),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene(strengthen_flash_scn),
                EventAction::ActivateScene(strengthen_scn),
            )
            .map_err(EngineError::Other)?;
            self.pending_chars.push(id);
        }
        Ok(())
    }
}
impl Crumble {
    pub(super) fn step_scene(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.stage != Stage::Complete {
            match self.stage {
                Stage::Falling => {
                    if !self.pending_chars.is_empty() {
                        if self.fall_delay == 0 {
                            // Determine the size of the next group of falling characters
                            let fall_group_size = ctx.rng.randint(1, self.fall_group_maxsize);
                            // Add the next group of falling characters to the animating characters list
                            for _ in 0..fall_group_size {
                                if !self.pending_chars.is_empty() {
                                    let next_char = self.pending_chars.remove(0);
                                    ctx.activate_scene(self, next_char, "weaken");
                                    ctx.active_characters.insert(next_char);
                                }
                            }
                            // Reset the fall delay and adjust the fall group size and delay range
                            self.fall_delay =
                                ctx.rng.randint(self.min_fall_delay, self.max_fall_delay);
                            if ctx.rng.randint(1, 10) > 4 {
                                // 60% chance to modify the fall delay and group size
                                self.fall_group_maxsize += 1;
                                self.min_fall_delay = std::cmp::max(0, self.min_fall_delay - 1);
                                self.max_fall_delay = std::cmp::max(0, self.max_fall_delay - 1);
                            }
                        } else {
                            self.fall_delay -= 1;
                        }
                    }
                    if self.pending_chars.is_empty() && ctx.active_characters.is_empty() {
                        self.stage = Stage::Vacuuming;
                    }
                }
                Stage::Vacuuming => {
                    if !self.unvacuumed_chars.is_empty() {
                        for _ in 0..ctx.rng.randint(3, 10) {
                            if !self.unvacuumed_chars.is_empty() {
                                let next_char = self.unvacuumed_chars.remove(0);
                                ctx.activate_path(self, next_char, "top");
                                ctx.active_characters.insert(next_char);
                            }
                        }
                    }
                    if ctx.active_characters.is_empty() {
                        self.stage = Stage::Resetting;
                    }
                }
                Stage::Resetting => {
                    if !self.reset {
                        let characters = {
                            let filter = CharacterFilter::default();
                            ctx.terminal.get_characters(
                                &mut ctx.rng,
                                filter,
                                CharacterSort::TopToBottomLeftToRight,
                            )
                        };
                        for id in characters {
                            ctx.activate_path(self, id, "input");
                            ctx.active_characters.insert(id);
                        }
                        self.reset = true;
                    }
                    if ctx.active_characters.is_empty() {
                        self.stage = Stage::Complete;
                    }
                }
                Stage::Complete => {}
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}
impl Default for CrumbleConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Crumble {
    fn default() -> Self {
        Self::new(CrumbleConfig::default())
    }
}
