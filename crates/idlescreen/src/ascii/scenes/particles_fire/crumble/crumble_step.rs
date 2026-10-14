// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Crumble {
    pub(super) fn build_scene(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // CrumbleIterator.DYNAMIC_NEUTRAL_GRAY
        let dynamic_neutral_gray = Color::from_hex("#808080").must_val();
        let white = Color::from_hex("#ffffff").must_val();

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

        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        let canvas_bottom = ctx.terminal.canvas.bottom;
        let canvas_top = ctx.terminal.canvas.top;
        let canvas_center_column = ctx.terminal.canvas.center_column;
        let canvas_center_row = ctx.terminal.canvas.center_row;
        for &id in &characters {
            let (input_coord, input_symbol, input_fg, input_bg, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.animation.input_fg_color.clone(),
                    ch.animation.input_bg_color.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            self.character_final_color_map.insert(
                id,
                final_gradient_mapping.get(&input_coord).must_val().clone(),
            );
            let (
                weak_fg_color,
                weak_bg_color,
                dust_fg_color,
                dust_bg_color,
                strengthen_flash_fg_gradient,
                strengthen_flash_bg_gradient,
                strengthen_fg_gradient,
                strengthen_bg_gradient,
            );
            if dynamic {
                let base_fg = input_fg
                    .as_ref()
                    .or_else(|| input_bg.is_none().then_some(&dynamic_neutral_gray));
                weak_fg_color = base_fg.map(|fg| Animation::adjust_color_brightness(fg, 0.65));
                weak_bg_color = input_bg
                    .as_ref()
                    .map(|bg| Animation::adjust_color_brightness(bg, 0.65));
                dust_fg_color = base_fg.map(|fg| Animation::adjust_color_brightness(fg, 0.55));
                dust_bg_color = input_bg
                    .as_ref()
                    .map(|bg| Animation::adjust_color_brightness(bg, 0.55));
                strengthen_flash_fg_gradient = base_fg
                    .map(|fg| {
                        Gradient::with_steps(&[fg.clone(), white.clone()], 6, false)
                            .map_err(EngineError::Other)
                    })
                    .transpose()?;
                strengthen_flash_bg_gradient = input_bg
                    .as_ref()
                    .map(|bg| {
                        Gradient::with_steps(&[bg.clone(), white.clone()], 6, false)
                            .map_err(EngineError::Other)
                    })
                    .transpose()?;
                strengthen_fg_gradient = input_fg
                    .as_ref()
                    .map(|fg| {
                        Gradient::with_steps(&[white.clone(), fg.clone()], 9, false)
                            .map_err(EngineError::Other)
                    })
                    .transpose()?;
                strengthen_bg_gradient = input_bg
                    .as_ref()
                    .map(|bg| {
                        Gradient::with_steps(&[white.clone(), bg.clone()], 9, false)
                            .map_err(EngineError::Other)
                    })
                    .transpose()?;
            } else {
                let final_color = self.character_final_color_map.get(&id).must_val().clone();
                weak_fg_color = Some(Animation::adjust_color_brightness(&final_color, 0.65));
                weak_bg_color = None;
                dust_fg_color = Some(Animation::adjust_color_brightness(&final_color, 0.55));
                dust_bg_color = None;
                strengthen_flash_fg_gradient = Some(
                    Gradient::with_steps(&[final_color.clone(), white.clone()], 6, false)
                        .map_err(EngineError::Other)?,
                );
                strengthen_flash_bg_gradient = None;
                strengthen_fg_gradient = Some(
                    Gradient::with_steps(&[white.clone(), final_color.clone()], 9, false)
                        .map_err(EngineError::Other)?,
                );
                strengthen_bg_gradient = None;
            }
            let weaken_fg_gradient = match (&weak_fg_color, &dust_fg_color) {
                (Some(weak), Some(dust)) => Some(
                    Gradient::with_steps(&[weak.clone(), dust.clone()], 9, false)
                        .map_err(EngineError::Other)?,
                ),
                _ => None,
            };
            let weaken_bg_gradient = match (&weak_bg_color, &dust_bg_color) {
                (Some(weak), Some(dust)) => Some(
                    Gradient::with_steps(&[weak.clone(), dust.clone()], 9, false)
                        .map_err(EngineError::Other)?,
                ),
                _ => None,
            };
            ctx.terminal.set_character_visibility(id, true);
            // set up initial and falling stage
            let initial_scn = {
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
                            colors: Some(ColorPair::new(
                                weak_fg_color.clone(),
                                weak_bg_color.clone(),
                            )),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                scene_id
            };
            ctx.activate_scene(self, id, &initial_scn);
            let fall_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let path_id = ch
                    .motion
                    .new_path(0.95, Some(Easing::OutBounce), None, 0, false, "")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .must_val()
                    .new_waypoint(Coord::new(input_coord.column, canvas_bottom), None, "")
                    .map_err(EngineError::Other)?;
                path_id
            };
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let weaken_scn = ch
                    .animation
                    .new_scene(false, None, None, "weaken", uses_pre);
                ch.animation
                    .scenes
                    .get_mut(&weaken_scn)
                    .must_val()
                    .apply_gradient_to_symbols(
                        &[input_symbol.clone()],
                        4,
                        weaken_fg_gradient.as_ref(),
                        weaken_bg_gradient.as_ref(),
                    )
                    .map_err(EngineError::Other)?;
            }
            self.setup_crumble_stage(
                ctx,
                id,
                &input_symbol,
                input_coord,
                uses_pre,
                dynamic && input_fg.is_none() && input_bg.is_none(),
                fall_path,
                (dust_fg_color, dust_bg_color),
                (strengthen_flash_fg_gradient, strengthen_flash_bg_gradient),
                (strengthen_fg_gradient, strengthen_bg_gradient),
            )?;
        }
        ctx.rng.shuffle(&mut self.pending_chars);
        self.fall_delay = 4;
        self.max_fall_delay = 4;
        self.min_fall_delay = 2;
        self.reset = false;
        self.fall_group_maxsize = 3;
        self.stage = Stage::Falling;
        self.unvacuumed_chars = ctx.terminal.input_characters.clone();
        ctx.rng.shuffle(&mut self.unvacuumed_chars);
        Ok(())
    }
}
impl Effect for Crumble {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.build_scene(ctx)
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        self.step_scene(ctx)
    }
}
