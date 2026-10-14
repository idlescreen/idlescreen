// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Effect for Print {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // PrintIterator.__init__: the typing head is added before build().
        self.typing_head = ctx.terminal.add_character("█", Coord::new(1, 1));

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
        let filter = CharacterFilter {
            inner_fill_chars: true,
            outer_fill_chars: true,
            ..Default::default()
        };
        let characters = ctx.terminal.get_characters(
            &mut ctx.rng,
            filter,
            ttfx::engine::terminal::CharacterSort::TopToBottomLeftToRight,
        );
        for &id in &characters {
            let final_colors = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                if dynamic {
                    ColorPair::new(
                        ch.animation.input_fg_color.clone(),
                        ch.animation.input_bg_color.clone(),
                    )
                } else {
                    ColorPair::new(
                        Some(
                            final_gradient_mapping
                                .get(&ch.input_coord)
                                .cloned()
                                .unwrap_or_else(|| Color::from_hex("ffffff").must_val()),
                        ),
                        None,
                    )
                }
            };
            self.character_final_color_map.insert(id, final_colors);
        }
        let input_rows = ctx
            .terminal
            .get_characters_grouped(filter, CharacterGroup::RowTopToBottom);
        for input_row in input_rows {
            let row = self.make_row(ctx, input_row)?;
            self.pending_rows.push(row);
        }
        self.current_row = self.pending_rows.remove(0);
        self.typing = true;
        self.last_column = 0;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !ctx.active_characters.is_empty() || self.typing {
            if ctx.terminal.arena[self.typing_head.0 as usize]
                .motion
                .active_path
                .is_some()
            {
                // print head is performing a carriage return
            } else if !self.current_row.untyped_chars.is_empty() {
                let count = std::cmp::min(
                    self.current_row.untyped_chars.len() as i64,
                    self.config.print_speed,
                );
                for _ in 0..count {
                    if let Some(next_char) = self.current_row.type_char() {
                        ctx.terminal.set_character_visibility(next_char, true);
                        ctx.active_characters.insert(next_char);
                        self.last_column =
                            ctx.terminal.arena[next_char.0 as usize].input_coord.column;
                    }
                }
            } else {
                let finished_row = std::mem::replace(
                    &mut self.current_row,
                    Row {
                        untyped_chars: Vec::new(),
                        typed_chars: Vec::new(),
                    },
                );
                self.processed_rows.push(finished_row);
                if !self.pending_rows.is_empty() {
                    for row in &self.processed_rows {
                        row.move_up(ctx);
                    }
                    self.current_row = self.pending_rows.remove(0);
                    let last_row_all_fill = self
                        .processed_rows
                        .last()
                        .must_val()
                        .typed_chars
                        .iter()
                        .all(|&id| ctx.terminal.arena[id.0 as usize].is_fill_character);
                    let current_row_all_fill = self
                        .current_row
                        .untyped_chars
                        .iter()
                        .all(|&id| ctx.terminal.arena[id.0 as usize].is_fill_character);
                    if !last_row_all_fill && !current_row_all_fill {
                        let left_extent = self
                            .current_row
                            .untyped_chars
                            .iter()
                            .copied()
                            .filter(|&id| !ctx.terminal.arena[id.0 as usize].is_fill_character)
                            .map(|id| ctx.terminal.arena[id.0 as usize].input_coord.column)
                            .min()
                            .must_val();
                        let text_right = ctx.terminal.canvas.text_right;
                        let arena = &ctx.terminal.arena;
                        self.current_row.untyped_chars.retain(|&id| {
                            let column = arena[id.0 as usize].input_coord.column;
                            left_extent <= column && column <= text_right
                        });
                    }
                    {
                        let head = &mut ctx.terminal.arena[self.typing_head.0 as usize];
                        head.motion.set_coordinate(Coord::new(self.last_column, 1));
                    }
                    ctx.terminal
                        .set_character_visibility(self.typing_head, true);
                    let target_column = ctx.terminal.arena
                        [self.current_row.untyped_chars[0].0 as usize]
                        .input_coord
                        .column;
                    {
                        let head = &mut ctx.terminal.arena[self.typing_head.0 as usize];
                        head.motion.paths.clear();
                        let path_id = head
                            .motion
                            .new_path(
                                self.config.print_head_return_speed,
                                Some(self.config.print_head_easing),
                                None,
                                0,
                                false,
                                "carriage_return_path",
                            )
                            .must_val();
                        head.motion
                            .paths
                            .get_mut(&path_id)
                            .must_val()
                            .new_waypoint(Coord::new(target_column, 1), None, "")
                            .must_val();
                    }
                    let typing_head = self.typing_head;
                    ctx.activate_path(self, typing_head, "carriage_return_path");
                    // contextlib.suppress(DuplicateEventRegistrationError):
                    // the same (event, path-id, callback) tuple re-registers
                    // every row and is rejected after the first.
                    let _ = ctx.register_event(
                        self.typing_head,
                        Event::PathComplete,
                        CallerKey::Path("carriage_return_path".to_string()),
                        EventAction::Callback(EffectCallback {
                            id: SET_INVISIBLE_CALLBACK,
                            args: Vec::new(),
                        }),
                    );
                    ctx.active_characters.insert(self.typing_head);
                } else {
                    self.typing = false;
                }
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}
impl Default for PrintConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Print {
    fn default() -> Self {
        Self::new(PrintConfig::default())
    }
}
