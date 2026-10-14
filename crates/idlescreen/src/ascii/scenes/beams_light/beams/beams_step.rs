// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Beams {
    pub(super) fn build_scene(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // __init__ precomputation (no RNG)
        self.final_wipe_groups = ctx.terminal.get_characters_grouped(
            CharacterFilter::default(),
            CharacterGroup::DiagonalTopLeftToBottomRight,
        );

        let all_chars_filter = CharacterFilter {
            input_chars: true,
            inner_fill_chars: true,
            outer_fill_chars: true,
            added_chars: false,
        };
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
        let characters = ctx.terminal.get_characters(
            &mut ctx.rng,
            all_chars_filter,
            CharacterSort::TopToBottomLeftToRight,
        );
        for id in characters {
            let ch = &ctx.terminal.arena[id.0 as usize];
            if ch.is_fill_character {
                self.character_final_color_map.insert(
                    id,
                    ColorPair::new(Some(Color::from_hex("#000000").must_val()), None),
                );
                continue;
            }
            let final_colors = if dynamic {
                ColorPair::new(
                    ch.animation.input_fg_color.clone(),
                    ch.animation.input_bg_color.clone(),
                )
            } else {
                ColorPair::new(
                    Some(
                        final_gradient_mapping
                            .get(&ch.input_coord)
                            .must_val()
                            .clone(),
                    ),
                    None,
                )
            };
            self.character_final_color_map.insert(id, final_colors);
        }

        let beam_gradient = Gradient::new(
            &self.config.beam_gradient_stops,
            &self.config.beam_gradient_steps,
            false,
            false,
        )
        .map_err(EngineError::Other)?;
        let mut groups: Vec<Group> = Vec::new();
        for row in ctx
            .terminal
            .get_characters_grouped(all_chars_filter, CharacterGroup::RowTopToBottom)
        {
            groups.push(self.make_group(ctx, row, Direction::Row));
        }
        for column in ctx
            .terminal
            .get_characters_grouped(all_chars_filter, CharacterGroup::ColumnLeftToRight)
        {
            groups.push(self.make_group(ctx, column, Direction::Column));
        }
        // Rows and columns contain the same characters; initialize each character's scenes only once.
        for group in groups
            .iter()
            .filter(|group| group.direction == Direction::Row)
        {
            for &id in &group.characters {
                let (input_symbol, uses_pre) = {
                    let ch = &ctx.terminal.arena[id.0 as usize];
                    (ch.input_symbol.clone(), ch.uses_input_preexisting_colors)
                };
                {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation
                        .new_scene(false, None, None, "beam_row", uses_pre);
                    ch.animation
                        .new_scene(false, None, None, "beam_column", uses_pre);
                    ch.animation
                        .new_scene(false, None, None, "brighten", uses_pre);
                    ch.animation
                        .scenes
                        .get_mut("beam_row")
                        .must_val()
                        .apply_gradient_to_symbols(
                            &self.config.beam_row_symbols,
                            self.config.beam_gradient_frames,
                            Some(&beam_gradient),
                            None,
                        )
                        .map_err(EngineError::Other)?;
                    ch.animation
                        .scenes
                        .get_mut("beam_column")
                        .must_val()
                        .apply_gradient_to_symbols(
                            &self.config.beam_column_symbols,
                            self.config.beam_gradient_frames,
                            Some(&beam_gradient),
                            None,
                        )
                        .map_err(EngineError::Other)?;
                }
                let char_colors = self.character_final_color_map.get(&id).must_val().clone();
                let mut fg_fade_gradient: Option<Gradient> = None;
                let mut bg_fade_gradient: Option<Gradient> = None;
                let mut fg_brighten_gradient: Option<Gradient> = None;
                let mut bg_brighten_gradient: Option<Gradient> = None;
                if let Some(fg) = &char_colors.fg_color {
                    let faded_fg_color = Animation::adjust_color_brightness(fg, 0.3);
                    fg_fade_gradient = Some(
                        Gradient::with_steps(&[fg.clone(), faded_fg_color.clone()], 10, false)
                            .map_err(EngineError::Other)?,
                    );
                    fg_brighten_gradient = Some(
                        Gradient::with_steps(&[faded_fg_color, fg.clone()], 10, false)
                            .map_err(EngineError::Other)?,
                    );
                }
                if let Some(bg) = &char_colors.bg_color {
                    let faded_bg_color = Animation::adjust_color_brightness(bg, 0.3);
                    bg_fade_gradient = Some(
                        Gradient::with_steps(&[bg.clone(), faded_bg_color.clone()], 10, false)
                            .map_err(EngineError::Other)?,
                    );
                    bg_brighten_gradient = Some(
                        Gradient::with_steps(&[faded_bg_color, bg.clone()], 10, false)
                            .map_err(EngineError::Other)?,
                    );
                }

                let ch = &mut ctx.terminal.arena[id.0 as usize];
                if fg_fade_gradient.is_some() || bg_fade_gradient.is_some() {
                    ch.animation
                        .scenes
                        .get_mut("beam_row")
                        .must_val()
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            2,
                            fg_fade_gradient.as_ref(),
                            bg_fade_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                    ch.animation
                        .scenes
                        .get_mut("beam_column")
                        .must_val()
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            2,
                            fg_fade_gradient.as_ref(),
                            bg_fade_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    ch.animation
                        .scenes
                        .get_mut("beam_row")
                        .must_val()
                        .add_frame(
                            &input_symbol,
                            2,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                    ch.animation
                        .scenes
                        .get_mut("beam_column")
                        .must_val()
                        .add_frame(
                            &input_symbol,
                            2,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }

                if fg_brighten_gradient.is_some() || bg_brighten_gradient.is_some() {
                    ch.animation
                        .scenes
                        .get_mut("brighten")
                        .must_val()
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            self.config.final_gradient_frames,
                            fg_brighten_gradient.as_ref(),
                            bg_brighten_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    ch.animation
                        .scenes
                        .get_mut("brighten")
                        .must_val()
                        .add_frame(
                            &input_symbol,
                            self.config.final_gradient_frames,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }
        }

        self.pending_groups = groups;
        ctx.rng.shuffle(&mut self.pending_groups);
        Ok(())
    }
}
