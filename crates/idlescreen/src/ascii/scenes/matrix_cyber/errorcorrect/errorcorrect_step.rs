// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    BLOCK_WIPE_END, BLOCK_WIPE_START, CallerKey, CharId, Color, ColorPair, EffectCallback,
    EffectHooks, EngineCtx, EngineError, ErrorCorrect, Event, EventAction, ExistingColorHandling,
    Gradient, HashMap, SyncMetric, VisualParams, hex_color,
};

impl ErrorCorrect {
    /// ErrorCorrectIterator._configure_swapped_character.
    pub(super) fn configure_swapped_character(
        &mut self,
        ctx: &mut EngineCtx,
        id: CharId,
        correcting_gradient: &Gradient,
    ) -> Result<(), EngineError> {
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let (input_symbol, uses_pre) = {
            let ch = &ctx.terminal.arena[id.0 as usize];
            (ch.input_symbol.clone(), ch.uses_input_preexisting_colors)
        };
        let (first_block_wipe, last_block_wipe) = {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let first = ch.animation.new_scene(false, None, None, "", uses_pre);
            let last = ch.animation.new_scene(false, None, None, "", uses_pre);
            (first, last)
        };
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let scene = ch
                .animation
                .scenes
                .get_mut(&first_block_wipe)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            for block in BLOCK_WIPE_START {
                scene
                    .add_frame(
                        block,
                        3,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(self.config.error_color), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
        }
        let final_colors = *self
            .character_final_color_map
            .get(&id)
            .ok_or_else(|| EngineError::Other(String::new()))?;
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let scene = ch
                .animation
                .scenes
                .get_mut(&last_block_wipe)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            if dynamic {
                for block in &BLOCK_WIPE_END[..BLOCK_WIPE_END.len() - 1] {
                    scene
                        .add_frame(
                            block,
                            3,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(self.config.correct_color), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                scene
                    .add_frame(
                        BLOCK_WIPE_END[BLOCK_WIPE_END.len() - 1],
                        3,
                        VisualParams {
                            colors: Some(final_colors),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            } else {
                for block in BLOCK_WIPE_END {
                    scene
                        .add_frame(
                            block,
                            3,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(self.config.correct_color), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }
        }
        let initial_scene = {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
            ch.animation
                .scenes
                .get_mut(&scene_id)
                .ok_or_else(|| EngineError::Other(String::new()))?
                .add_frame(
                    &input_symbol,
                    1,
                    VisualParams {
                        colors: Some(ColorPair::new(Some(self.config.error_color), None)),
                        ..Default::default()
                    },
                )
                .map_err(EngineError::Other)?;
            scene_id
        };
        ctx.activate_scene(self, id, &initial_scene);
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let error_scene = ch.animation.new_scene(false, None, None, "error", uses_pre);
            let scene = ch
                .animation
                .scenes
                .get_mut(&error_scene)
                .ok_or_else(|| EngineError::Other(String::new()))?;
            for _ in 0..10 {
                scene
                    .add_frame(
                        "▓",
                        3,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(self.config.error_color), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                scene
                    .add_frame(
                        &input_symbol,
                        3,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(hex_color("ffffff")), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
        }
        let correcting_scene = {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let scene_id =
                ch.animation
                    .new_scene(false, Some(SyncMetric::Distance), None, "", uses_pre);
            ch.animation
                .scenes
                .get_mut(&scene_id)
                .ok_or_else(|| EngineError::Other(String::new()))?
                .apply_gradient_to_symbols(&["█".to_string()], 3, Some(correcting_gradient), None)
                .map_err(EngineError::Other)?;
            scene_id
        };
        let final_scene = if dynamic {
            self.get_dynamic_final_scene(ctx, id)?
        } else {
            let final_fg = final_colors_fg(&self.character_final_color_map, id);
            let char_final_gradient =
                Gradient::with_steps(&[self.config.correct_color, final_fg], 10, false)
                    .map_err(EngineError::Other)?;
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
            ch.animation
                .scenes
                .get_mut(&scene_id)
                .ok_or_else(|| EngineError::Other(String::new()))?
                .apply_gradient_to_symbols(
                    std::slice::from_ref(&input_symbol),
                    3,
                    Some(&char_final_gradient),
                    None,
                )
                .map_err(EngineError::Other)?;
            scene_id
        };
        ctx.register_event(
            id,
            Event::SceneComplete,
            CallerKey::Scene("error".to_string()),
            EventAction::ActivateScene(first_block_wipe.clone()),
        )
        .map_err(EngineError::Other)?;
        ctx.register_event(
            id,
            Event::SceneComplete,
            CallerKey::Scene(first_block_wipe.clone()),
            EventAction::ActivateScene(correcting_scene),
        )
        .map_err(EngineError::Other)?;
        ctx.register_event(
            id,
            Event::SceneComplete,
            CallerKey::Scene(first_block_wipe),
            EventAction::ActivatePath("input_coord".to_string()),
        )
        .map_err(EngineError::Other)?;
        ctx.register_event(
            id,
            Event::PathActivated,
            CallerKey::Path("input_coord".to_string()),
            EventAction::SetLayer(1),
        )
        .map_err(EngineError::Other)?;
        ctx.register_event(
            id,
            Event::PathComplete,
            CallerKey::Path("input_coord".to_string()),
            EventAction::SetLayer(0),
        )
        .map_err(EngineError::Other)?;
        ctx.register_event(
            id,
            Event::PathComplete,
            CallerKey::Path("input_coord".to_string()),
            EventAction::ActivateScene(last_block_wipe.clone()),
        )
        .map_err(EngineError::Other)?;
        ctx.register_event(
            id,
            Event::SceneComplete,
            CallerKey::Scene(last_block_wipe),
            EventAction::ActivateScene(final_scene),
        )
        .map_err(EngineError::Other)?;
        Ok(())
    }
}
pub(super) fn final_colors_fg(map: &HashMap<CharId, ColorPair>, id: CharId) -> Color {
    map.get(&id)
        .and_then(|cp| cp.fg_color)
        .unwrap_or_else(|| hex_color("ffffff"))
}
impl EffectHooks for ErrorCorrect {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
