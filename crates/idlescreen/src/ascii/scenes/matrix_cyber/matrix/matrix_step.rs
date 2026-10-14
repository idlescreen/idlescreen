// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl RainColumn {
    /// RainColumn.__init__ (calls setup_column("rain")).
    pub(super) fn new(ctx: &mut EngineCtx, config: &MatrixConfig, characters: Vec<CharId>) -> Self {
        let mut column = RainColumn {
            characters,
            pending_characters: Vec::new(),
            visible_characters: Vec::new(),
            phase: ColumnPhase::Rain,
            column_drop_chance: 0.08,
            base_rain_fall_delay: 0,
            active_rain_fall_delay: 0,
            length: 0,
            hold_time: 0,
        };
        column.setup_column(ctx, config, ColumnPhase::Rain);
        column
    }

    /// RainColumn.setup_column.
    pub(super) fn setup_column(
        &mut self,
        ctx: &mut EngineCtx,
        config: &MatrixConfig,
        phase: ColumnPhase,
    ) {
        self.pending_characters.clear();
        self.phase = phase;
        for &character in &self.characters {
            ctx.terminal.set_character_visibility(character, false);
            self.pending_characters.push(character);
            let ch = &mut ctx.terminal.arena[character.0 as usize];
            ch.motion.current_coord = ch.input_coord;
        }
        self.visible_characters = Vec::new();
        self.base_rain_fall_delay = if self.phase == ColumnPhase::Fill {
            ctx.rng.randint(
                std::cmp::max(floor_div(config.rain_fall_delay_range.0, 3), 1),
                std::cmp::max(floor_div(config.rain_fall_delay_range.1, 3), 1),
            )
        } else {
            ctx.rng.randint(
                config.rain_fall_delay_range.0,
                config.rain_fall_delay_range.1,
            )
        };
        self.active_rain_fall_delay = 0;
        self.length = if self.phase == ColumnPhase::Rain {
            ctx.rng.randint(
                std::cmp::max(1, (self.characters.len() as f64 * 0.1) as i64),
                self.characters.len() as i64,
            ) as usize
        } else {
            self.characters.len()
        };
        self.hold_time = 0;
        if self.length == self.characters.len() {
            self.hold_time = ctx.rng.randint(20, 45);
        }
    }

    /// RainColumn.trim_column.
    pub(super) fn trim_column(&mut self, ctx: &mut EngineCtx, rain_colors: &[Color]) {
        if self.visible_characters.is_empty() {
            return;
        }
        let popped_char = self.visible_characters.remove(0);
        ctx.terminal.set_character_visibility(popped_char, false);
        if self.visible_characters.len() > 1 {
            self.fade_last_character(ctx, rain_colors);
        }
    }

    /// RainColumn.drop_column.
    pub(super) fn drop_column(&mut self, ctx: &mut EngineCtx) {
        let canvas_bottom = ctx.terminal.canvas.bottom;
        let mut out_of_canvas: Vec<CharId> = Vec::new();
        for &character in &self.visible_characters {
            let new_coord = {
                let motion = &mut ctx.terminal.arena[character.0 as usize].motion;
                let current = motion.current_coord;
                motion.current_coord = Coord::new(current.column, current.row - 1);
                motion.current_coord
            };
            if new_coord.row < canvas_bottom {
                ctx.terminal.set_character_visibility(character, false);
                out_of_canvas.push(character);
            }
        }
        self.visible_characters
            .retain(|ch| !out_of_canvas.contains(ch));
    }

    /// RainColumn.fade_last_character.
    pub(super) fn fade_last_character(&mut self, ctx: &mut EngineCtx, rain_colors: &[Color]) {
        // random.choice(self.rain_colors[-3:])
        let tail = &rain_colors[rain_colors.len().saturating_sub(3)..];
        let darker_color = Animation::adjust_color_brightness(ctx.rng.choice(tail), 0.65);
        let target = self.visible_characters[0];
        let symbol = ctx.terminal.arena[target.0 as usize]
            .animation
            .current_character_visual
            .symbol
            .clone();
        set_appearance(
            ctx,
            target,
            &symbol,
            ColorPair::new(Some(darker_color), None),
        );
    }

    /// RainColumn.resolve_char.
    pub(super) fn resolve_char(&mut self, ctx: &mut EngineCtx) -> CharId {
        let index = ctx.rng.randint(0, self.visible_characters.len() as i64 - 1) as usize;
        self.visible_characters.remove(index)
    }
}
impl RainColumn {
    /// RainColumn.tick.
    pub(super) fn tick(
        &mut self,
        ctx: &mut EngineCtx,
        config: &MatrixConfig,
        rain_colors: &[Color],
    ) {
        if self.active_rain_fall_delay == 0 {
            if !self.pending_characters.is_empty() {
                let next_char = self.pending_characters.remove(0);
                let symbol = ctx.rng.choice(&config.rain_symbols).clone();
                set_appearance(
                    ctx,
                    next_char,
                    &symbol,
                    ColorPair::new(Some(config.highlight_color.clone()), None),
                );
                let previous_character = self.visible_characters.last().copied();
                // if there is a previous character, remove the highlight
                if let Some(previous_character) = previous_character {
                    let prev_symbol = ctx.terminal.arena[previous_character.0 as usize]
                        .animation
                        .current_character_visual
                        .symbol
                        .clone();
                    let fg = ctx.rng.choice(rain_colors).clone();
                    set_appearance(
                        ctx,
                        previous_character,
                        &prev_symbol,
                        ColorPair::new(Some(fg), None),
                    );
                }
                ctx.terminal.set_character_visibility(next_char, true);
                self.visible_characters.push(next_char);
            } else if !self.visible_characters.is_empty() {
                // adjust the bottom character color to remove the highlight
                let last_char = *self.visible_characters.last().must_val();
                let last_is_highlight = {
                    let visual = &ctx.terminal.arena[last_char.0 as usize]
                        .animation
                        .current_character_visual;
                    visual.colors.as_ref().is_some_and(|colors| {
                        colors.fg_color.as_ref() == Some(&config.highlight_color)
                    })
                };
                if last_is_highlight {
                    let symbol = ctx.terminal.arena[last_char.0 as usize]
                        .animation
                        .current_character_visual
                        .symbol
                        .clone();
                    let fg = ctx.rng.choice(rain_colors).clone();
                    set_appearance(ctx, last_char, &symbol, ColorPair::new(Some(fg), None));
                }

                if self.hold_time != 0 {
                    self.hold_time -= 1;
                } else if self.phase == ColumnPhase::Rain {
                    if ctx.rng.random() < self.column_drop_chance {
                        self.drop_column(ctx);
                    }
                    self.trim_column(ctx, rain_colors);
                }
            }

            // if the column is longer than the preset length while still adding characters, trim it
            if self.visible_characters.len() > self.length {
                self.trim_column(ctx, rain_colors);
            }
            self.active_rain_fall_delay = self.base_rain_fall_delay;
        } else {
            self.active_rain_fall_delay -= 1;
        }

        // randomly change the symbol and/or color of the characters
        for &character in &self.visible_characters {
            // Draw both chances (and any resulting choices) in upstream order,
            // but leave the cached visual alone when neither value changes.
            let next_symbol = if ctx.rng.random() < config.symbol_swap_chance {
                Some(ctx.rng.choice(&config.rain_symbols).as_str())
            } else {
                None
            };
            let next_color = if ctx.rng.random() < config.color_swap_chance {
                Some(ctx.rng.choice(rain_colors))
            } else {
                None
            };
            if next_symbol.is_none() && next_color.is_none() {
                continue;
            }

            let values_unchanged = {
                let visual = &ctx.terminal.arena[character.0 as usize]
                    .animation
                    .current_character_visual;
                next_symbol.is_none_or(|symbol| symbol == visual.symbol)
                    && next_color.is_none_or(|color| {
                        visual
                            .colors
                            .as_ref()
                            .and_then(|colors| colors.fg_color.as_ref())
                            == Some(color)
                    })
            };
            if values_unchanged {
                continue;
            }

            let (sym, col) = {
                let vis = &ctx.terminal.arena[character.0 as usize]
                    .animation
                    .current_character_visual;
                let sym = next_symbol.map_or_else(|| vis.symbol.clone(), str::to_owned);
                let col = next_color
                    .cloned()
                    .or_else(|| vis.colors.as_ref().and_then(|c| c.fg_color.clone()));
                (sym, col)
            };
            set_appearance(ctx, character, &sym, ColorPair::new(col, None));
        }
    }
}
