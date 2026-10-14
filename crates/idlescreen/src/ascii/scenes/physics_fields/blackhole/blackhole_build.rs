// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Blackhole {
    /// BlackholeIterator.explode_singularity.
    pub(super) fn explode_singularity(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let star_colors = [
            Color::from_hex("#ffcc0d").must_val(),
            Color::from_hex("#ff7326").must_val(),
            Color::from_hex("#ff194d").must_val(),
            Color::from_hex("#bf2669").must_val(),
            Color::from_hex("#702a8c").must_val(),
            Color::from_hex("#049dbf").must_val(),
        ];
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
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
            let circle_coords = geometry::find_coords_on_circle(input_coord, 3, 5, true);
            let nearby_coord = circle_coords[ctx.rng.randrange(0, 5) as usize];
            let nearby_speed = ctx.rng.randint(6, 9) as f64 / 10.0;
            let nearby_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let path_id = ch
                    .motion
                    .new_path(nearby_speed, Some(Easing::OutExpo), None, 0, false, "")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .must_val()
                    .new_waypoint(nearby_coord, None, "")
                    .map_err(EngineError::Other)?;
                path_id
            };
            let input_speed = ctx.rng.randint(10, 16) as f64 / 100.0;
            let input_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let path_id = ch
                    .motion
                    .new_path(input_speed, Some(Easing::InCubic), None, 0, false, "")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .must_val()
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
                path_id
            };
            let explode_star_color = ctx.rng.choice(&star_colors).clone();
            let explode_scn = {
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
                            colors: Some(ColorPair::new(Some(explode_star_color.clone()), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                scene_id
            };
            let cooling_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "", uses_pre)
            };
            if dynamic && ctx.preexisting_colors_present {
                if input_fg.is_none() && input_bg.is_none() {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation
                        .scenes
                        .get_mut(&cooling_scn)
                        .must_val()
                        .add_frame(
                            &input_symbol,
                            1,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    let cooling_gradient_fg = match &input_fg {
                        Some(fg) => Some(
                            Gradient::with_steps(
                                &[explode_star_color.clone(), fg.clone()],
                                10,
                                false,
                            )
                            .map_err(EngineError::Other)?,
                        ),
                        None => None,
                    };
                    let cooling_gradient_bg = match &input_bg {
                        Some(bg) => Some(
                            Gradient::with_steps(
                                &[explode_star_color.clone(), bg.clone()],
                                10,
                                false,
                            )
                            .map_err(EngineError::Other)?,
                        ),
                        None => None,
                    };
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation
                        .scenes
                        .get_mut(&cooling_scn)
                        .must_val()
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            20,
                            cooling_gradient_fg.as_ref(),
                            cooling_gradient_bg.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                }
            } else {
                let final_color = self.character_final_color_map.get(&id).must_val().clone();
                let cooling_gradient =
                    Gradient::with_steps(&[explode_star_color.clone(), final_color], 10, false)
                        .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&cooling_scn)
                    .must_val()
                    .apply_gradient_to_symbols(
                        &[input_symbol.clone()],
                        20,
                        Some(&cooling_gradient),
                        None,
                    )
                    .map_err(EngineError::Other)?;
            }
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(nearby_path.clone()),
                EventAction::ActivatePath(input_path),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(nearby_path.clone()),
                EventAction::ActivateScene(cooling_scn),
            )
            .map_err(EngineError::Other)?;
            ctx.activate_scene(self, id, &explode_scn);
            ctx.activate_path(self, id, &nearby_path);
            ctx.active_characters.insert(id);
        }
        Ok(())
    }
}
impl EffectHooks for Blackhole {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
