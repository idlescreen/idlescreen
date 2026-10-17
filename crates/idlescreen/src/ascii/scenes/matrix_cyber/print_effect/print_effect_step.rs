// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CharId, ColorPair, Coord, EffectCallback, EffectHooks, EngineCtx, EngineError,
    ExistingColorHandling, Gradient, HashMap, Print, PrintConfig, Row, SET_INVISIBLE_CALLBACK,
    VisualParams, hex_color,
};

impl Print {
    pub fn new(config: PrintConfig) -> Self {
        Print {
            config,
            pending_rows: Vec::new(),
            processed_rows: Vec::new(),
            typing_head: CharId(0),
            character_final_color_map: HashMap::new(),
            current_row: Row {
                untyped_chars: Vec::new(),
                typed_chars: Vec::new(),
            },
            typing: false,
            last_column: 0,
        }
    }

    /// PrintIterator.Row.__init__.
    pub(super) fn make_row(
        &mut self,
        ctx: &mut EngineCtx,
        characters: Vec<CharId>,
    ) -> Result<Row, EngineError> {
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let typing_head_color = hex_color("ffffff");
        let all_spaces = characters
            .iter()
            .all(|&id| ctx.terminal.arena[id.0 as usize].input_symbol == " ");
        let characters: Vec<CharId> = if all_spaces {
            characters.into_iter().take(1).collect()
        } else {
            let right_extent = characters
                .iter()
                .copied()
                .filter(|&id| !ctx.terminal.arena[id.0 as usize].is_fill_character)
                .map(|id| ctx.terminal.arena[id.0 as usize].input_coord.column)
                .max()
                .ok_or_else(|| EngineError::Other(String::new()))?;
            characters
                .into_iter()
                .filter(|&id| ctx.terminal.arena[id.0 as usize].input_coord.column <= right_extent)
                .collect()
        };
        let mut untyped_chars: Vec<CharId> = Vec::new();
        for id in characters {
            let (input_symbol, input_column, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_symbol.clone(),
                    ch.input_coord.column,
                    ch.uses_input_preexisting_colors,
                )
            };
            let typed_animation = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion.set_coordinate(Coord::new(input_column, 1));
                ch.animation.new_scene(false, None, None, "", uses_pre)
            };
            if dynamic {
                let final_colors = *self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let fg_gradient = match &final_colors.fg_color {
                    Some(fg) => Some(
                        Gradient::with_steps(&[typing_head_color, *fg], 5, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let bg_gradient = match &final_colors.bg_color {
                    Some(bg) => Some(
                        Gradient::with_steps(&[typing_head_color, *bg], 5, false)
                            .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                if fg_gradient.is_some() || bg_gradient.is_some() {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation
                        .scenes
                        .get_mut(&typed_animation)
                        .ok_or_else(|| EngineError::Other(String::new()))?
                        .apply_gradient_to_symbols(
                            &[
                                "█".to_string(),
                                "▓".to_string(),
                                "▒".to_string(),
                                "░".to_string(),
                                input_symbol.clone(),
                            ],
                            3,
                            fg_gradient.as_ref(),
                            bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    let head_gradient =
                        Gradient::with_steps(&[typing_head_color, typing_head_color], 4, false)
                            .map_err(EngineError::Other)?;
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    let scene = ch
                        .animation
                        .scenes
                        .get_mut(&typed_animation)
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    scene
                        .apply_gradient_to_symbols(
                            &[
                                "█".to_string(),
                                "▓".to_string(),
                                "▒".to_string(),
                                "░".to_string(),
                            ],
                            3,
                            Some(&head_gradient),
                            None,
                        )
                        .map_err(EngineError::Other)?;
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
                }
            } else {
                let final_fg = self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .fg_color
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let color_gradient = Gradient::with_steps(&[typing_head_color, final_fg], 5, false)
                    .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&typed_animation)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .apply_gradient_to_symbols(
                        &[
                            "█".to_string(),
                            "▓".to_string(),
                            "▒".to_string(),
                            "░".to_string(),
                            input_symbol.clone(),
                        ],
                        3,
                        Some(&color_gradient),
                        None,
                    )
                    .map_err(EngineError::Other)?;
            }
            ctx.activate_scene(self, id, &typed_animation);
            untyped_chars.push(id);
        }
        Ok(Row {
            untyped_chars,
            typed_chars: Vec::new(),
        })
    }
}
impl EffectHooks for Print {
    fn dispatch_callback(
        &mut self,
        ctx: &mut EngineCtx,
        character: CharId,
        callback: &EffectCallback,
    ) {
        if callback.id == SET_INVISIBLE_CALLBACK {
            // EventHandler.Callback(self.terminal.set_character_visibility, False)
            ctx.terminal.set_character_visibility(character, false);
        }
    }
}
