// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CallerKey, CharId, ColorPair, Coord, EngineCtx, EngineError, Event, EventAction, SyncMetric,
    VhsTape, VisualParams, hex_color,
};

impl VhsTape {
    /// Line.build_line_effects — one offset/direction/hold_time draw per line,
    /// then paths, scenes (snow draws 25x2, final_snow 30x2), and events per
    /// character.
    pub(super) fn build_line_effects(
        &mut self,
        ctx: &mut EngineCtx,
        characters: &[CharId],
    ) -> Result<(), EngineError> {
        let glitch_line_colors = self.config.glitch_line_colors.clone();
        let snow_chars = ["#", "*", ".", ":"];
        let noise_colors = self.config.noise_colors.clone();
        let offset = ctx.rng.randint(4, 25);
        let direction = *ctx.rng.choice(&[-1i64, 1]);
        let hold_time = ctx.rng.randint(1, 50);
        for &id in characters {
            let (input_coord, input_symbol, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            let stable_colors = *self
                .character_stable_color_map
                .get(&id)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            let final_colors = *self
                .character_final_color_map
                .get(&id)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                for (name, hold, col) in [
                    ("glitch", hold_time, input_coord.column + offset * direction),
                    ("restore", 0, input_coord.column),
                    ("glitch_wave_mid", 0, input_coord.column + 8),
                    ("glitch_wave_end", 0, input_coord.column + 14),
                ] {
                    let p = ch
                        .motion
                        .new_path(2.0, None, None, hold, false, name)
                        .map_err(EngineError::Other)?;
                    ch.motion
                        .paths
                        .get_mut(&p)
                        .ok_or_else(|| EngineError::Other(String::new()))?
                        .new_waypoint(Coord::new(col, input_coord.row), None, name)
                        .map_err(EngineError::Other)?;
                }
            }
            // make glitch scenes
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let base_scn = ch.animation.new_scene(false, None, None, "base", uses_pre);
                ch.animation
                    .scenes
                    .get_mut(&base_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(stable_colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                let fwd_scn = ch.animation.new_scene(
                    false,
                    Some(SyncMetric::Step),
                    None,
                    "rgb_glitch_fwd",
                    uses_pre,
                );
                let scene = ch
                    .animation
                    .scenes
                    .get_mut(&fwd_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                for color in &glitch_line_colors {
                    scene
                        .add_frame(
                            &input_symbol,
                            1,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(*color), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                let bwd_scn = ch.animation.new_scene(
                    false,
                    Some(SyncMetric::Step),
                    None,
                    "rgb_glitch_bwd",
                    uses_pre,
                );
                let scene = ch
                    .animation
                    .scenes
                    .get_mut(&bwd_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                for color in glitch_line_colors.iter().rev() {
                    scene
                        .add_frame(
                            &input_symbol,
                            1,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(*color), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                ch.animation.new_scene(false, None, None, "snow", uses_pre);
            }
            for _ in 0..25 {
                let symbol = *ctx.rng.choice(&snow_chars);
                let color = *ctx.rng.choice(&noise_colors);
                ctx.terminal.arena[id.0 as usize]
                    .animation
                    .scenes
                    .get_mut("snow")
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        symbol,
                        2,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(color), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut("snow")
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(stable_colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                ch.animation
                    .new_scene(false, None, None, "final_snow", uses_pre);
                let redraw_scn =
                    ch.animation
                        .new_scene(false, None, None, "final_redraw", uses_pre);
                let scene = ch
                    .animation
                    .scenes
                    .get_mut(&redraw_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                scene
                    .add_frame(
                        "█",
                        6,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(hex_color("ffffff")), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                scene
                    .add_frame(
                        &input_symbol,
                        1,
                        VisualParams {
                            colors: Some(final_colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            for _ in 0..30 {
                let symbol = *ctx.rng.choice(&snow_chars);
                let color = *ctx.rng.choice(&noise_colors);
                ctx.terminal.arena[id.0 as usize]
                    .animation
                    .scenes
                    .get_mut("final_snow")
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        symbol,
                        2,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(color), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
            for (ev, key, act) in [
                (
                    Event::PathComplete,
                    CallerKey::Path("glitch".to_string()),
                    EventAction::ActivatePath("restore".to_string()),
                ),
                (
                    Event::PathActivated,
                    CallerKey::Path("glitch".to_string()),
                    EventAction::ActivateScene("rgb_glitch_fwd".to_string()),
                ),
                (
                    Event::PathActivated,
                    CallerKey::Path("restore".to_string()),
                    EventAction::ActivateScene("rgb_glitch_bwd".to_string()),
                ),
                (
                    Event::PathActivated,
                    CallerKey::Path("glitch_wave_mid".to_string()),
                    EventAction::ActivateScene("rgb_glitch_fwd".to_string()),
                ),
                (
                    Event::PathActivated,
                    CallerKey::Path("glitch_wave_end".to_string()),
                    EventAction::ActivateScene("rgb_glitch_fwd".to_string()),
                ),
                (
                    Event::SceneComplete,
                    CallerKey::Scene("rgb_glitch_bwd".to_string()),
                    EventAction::ActivateScene("base".to_string()),
                ),
            ] {
                ctx.register_event(id, ev, key, act)
                    .map_err(EngineError::Other)?;
            }
        }
        Ok(())
    }
}
