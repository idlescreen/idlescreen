// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CallerKey, CharId, Color, ColorPair, Coord, Easing, EngineCtx, EngineError, Event, EventAction,
    Gradient, Swarm, SyncMetric, VisualParams,
};

impl Swarm {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn setup_swarm_character(
        &mut self,
        ctx: &mut EngineCtx,
        id: CharId,
        swarm_spawn: Coord,
        swarm_gradient_mirror: &[Color],
        swarm_area_coordinate_map: &[(Coord, Vec<Coord>)],
        dynamic: bool,
        dynamic_clear_color: &Color,
    ) -> Result<(), EngineError> {
        let (input_coord, input_symbol, uses_pre) = {
            let ch = &ctx.terminal.arena[id.0 as usize];
            (
                ch.input_coord,
                ch.input_symbol.clone(),
                ch.uses_input_preexisting_colors,
            )
        };
        let flash_scn = {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.motion.set_coordinate(swarm_spawn);
            let scene_id =
                ch.animation
                    .new_scene(false, Some(SyncMetric::Distance), None, "", uses_pre);
            let scene = ch
                .animation
                .scenes
                .get_mut(&scene_id)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            for step in swarm_gradient_mirror {
                scene
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(*step), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            scene_id
        };
        for (swarm_area_count, (_, swarm_area_coords)) in
            swarm_area_coordinate_map.iter().enumerate()
        {
            let swarm_area_name = format!("{swarm_area_count}_swarm_area");
            let origin_waypoint_coord = *ctx.rng.choice(swarm_area_coords);
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let origin_path = ch
                    .motion
                    .new_path(0.7, Some(Easing::OutSine), None, 0, false, &swarm_area_name)
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&origin_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(origin_waypoint_coord, None, &swarm_area_name)
                    .map_err(EngineError::Other)?;
            }
            for action in [
                EventAction::ActivateScene(flash_scn.clone()),
                EventAction::SetLayer(1),
            ] {
                ctx.register_event(
                    id,
                    Event::PathActivated,
                    CallerKey::Path(swarm_area_name.clone()),
                    action,
                )
                .map_err(EngineError::Other)?;
            }
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(swarm_area_name.clone()),
                EventAction::DeactivateScene(None),
            )
            .map_err(EngineError::Other)?;
            for _ in 0..2 {
                let next_coord = *ctx.rng.choice(swarm_area_coords);
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let inner_path_id = ch.motion.paths.len().to_string();
                let inner_path = ch
                    .motion
                    .new_path(0.4, Some(Easing::InOutSine), None, 0, false, &inner_path_id)
                    .map_err(EngineError::Other)?;
                let waypoint_id = ch.motion.paths.len().to_string();
                ch.motion
                    .paths
                    .get_mut(&inner_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(next_coord, None, &waypoint_id)
                    .map_err(EngineError::Other)?;
            }
        }
        let input_path = {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let path_id = ch
                .motion
                .new_path(0.85, Some(Easing::InOutQuad), None, 0, false, "")
                .map_err(EngineError::Other)?;
            ch.motion
                .paths
                .get_mut(&path_id)
                .ok_or_else(|| EngineError::Other(String::new()))?
                .new_waypoint(input_coord, None, "")
                .map_err(EngineError::Other)?;
            path_id
        };
        let input_scn = ctx.terminal.arena[id.0 as usize]
            .animation
            .new_scene(false, None, None, "", uses_pre);
        let final_colors = *self
            .character_final_color_map
            .get(&id)
            .ok_or_else(|| EngineError::Other(String::new()))?;
        if dynamic {
            if final_colors.fg_color.is_none() && final_colors.bg_color.is_none() {
                let clear_gradient = Gradient::with_steps(
                    &[self.config.flash_color, *dynamic_clear_color],
                    10,
                    false,
                )
                .map_err(EngineError::Other)?;
                let scene = ctx.terminal.arena[id.0 as usize]
                    .animation
                    .scenes
                    .get_mut(&input_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                for step in &clear_gradient.spectrum {
                    scene
                        .add_frame(
                            &input_symbol,
                            3,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(*step), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                scene
                    .add_frame(
                        &input_symbol,
                        3,
                        VisualParams {
                            colors: Some(ColorPair::default()),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            } else {
                let step_grad = |opt: &Option<Color>| {
                    opt.as_ref()
                        .map(|c| {
                            Gradient::with_steps(&[self.config.flash_color, *c], 10, false)
                                .map_err(EngineError::Other)
                        })
                        .transpose()
                };
                let fg_gradient = step_grad(&final_colors.fg_color)?;
                let bg_gradient = step_grad(&final_colors.bg_color)?;
                ctx.terminal.arena[id.0 as usize]
                    .animation
                    .scenes
                    .get_mut(&input_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .apply_gradient_to_symbols(
                        std::slice::from_ref(&input_symbol),
                        3,
                        fg_gradient.as_ref(),
                        bg_gradient.as_ref(),
                    )
                    .map_err(EngineError::Other)?;
            }
        } else {
            let final_fg = final_colors
                .fg_color
                .ok_or_else(|| EngineError::Other(String::new()))?;
            let landing_gradient =
                Gradient::with_steps(&[self.config.flash_color, final_fg], 10, false)
                    .map_err(EngineError::Other)?;
            let scene = ctx.terminal.arena[id.0 as usize]
                .animation
                .scenes
                .get_mut(&input_scn)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            for step in &landing_gradient.spectrum {
                scene
                    .add_frame(
                        &input_symbol,
                        3,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(*step), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
        }
        for action in [
            EventAction::ActivateScene(input_scn),
            EventAction::SetLayer(0),
        ] {
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(input_path.clone()),
                action,
            )
            .map_err(EngineError::Other)?;
        }
        ctx.register_event(
            id,
            Event::PathActivated,
            CallerKey::Path(input_path),
            EventAction::ActivateScene(flash_scn),
        )
        .map_err(EngineError::Other)?;
        let all_paths: Vec<String> = ctx.terminal.arena[id.0 as usize]
            .motion
            .paths
            .keys()
            .map(std::string::ToString::to_string)
            .collect();
        ctx.chain_paths(id, &all_paths, false)
            .map_err(EngineError::Other)?;
        Ok(())
    }
}
