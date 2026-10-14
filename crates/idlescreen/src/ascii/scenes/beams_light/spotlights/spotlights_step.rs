// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Per-frame illumination and Effect implementation for Spotlights.

use ttfx::engine::animation::{Animation, ExistingColorHandling};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::geometry::{self, Coord};
use ttfx::utils::graphics::{ColorPair, Gradient};

use super::Spotlights;
use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};

impl Spotlights {
    pub(super) fn illuminate_chars(&mut self, ctx: &mut EngineCtx, range_: i64) {
        let mut chars_in_range = std::mem::take(&mut self.illuminated_scratch);
        chars_in_range.clear();
        let spotlight_coords: Vec<Coord> = self
            .spotlights
            .iter()
            .map(|&s| ctx.terminal.arena[s.0 as usize].motion.current_coord)
            .collect();

        for &current_coord in &spotlight_coords {
            for coord in geometry::find_coords_in_circle(current_coord, range_) {
                if let Some(id) = ctx.terminal.get_character_by_input_coord(coord)
                    && Self::is_spotlightable(ctx, id)
                {
                    chars_in_range.insert(id);
                }
            }
        }

        for id in self
            .illuminated_chars
            .iter()
            .filter(|id| !chars_in_range.contains(id))
        {
            let expand_override = self.get_expand_color_override(ctx, id);
            let colors = match expand_override {
                None => self
                    .character_color_map
                    .get(id.0 as usize)
                    .map_or_else(ColorPair::default, |p| p.1.clone()),
                Some(overridden) => overridden,
            };
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let uses_pre = ch.uses_input_preexisting_colors;
            ch.animation
                .set_appearance(&ch.input_symbol, uses_pre, None, Some(colors));
        }

        let falloff_start = range_ as f64 * (1.0 - self.config.beam_falloff);
        let falloff_span = (range_ as f64 * self.config.beam_falloff).max(f64::EPSILON);

        for id in &chars_in_range {
            let input_coord = ctx.terminal.arena[id.0 as usize].input_coord;
            let distance = spotlight_coords
                .iter()
                .map(|&sc| geometry::find_length_of_line(sc, input_coord, true))
                .fold(f64::INFINITY, f64::min);

            let bright_base = self
                .character_color_map
                .get(id.0 as usize)
                .map_or_else(ColorPair::default, |p| p.0.clone());

            let adjusted_color = if distance > falloff_start {
                let brightness_factor = (1.0 - (distance - falloff_start) / falloff_span).max(0.2);
                Self::adjust_color_pair_brightness(&bright_base, brightness_factor)
            } else {
                bright_base
            };
            let colors = self
                .get_expand_color_override(ctx, id)
                .unwrap_or(adjusted_color);
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let uses_pre = ch.uses_input_preexisting_colors;
            ch.animation
                .set_appearance(&ch.input_symbol, uses_pre, None, Some(colors));
        }
        self.illuminated_scratch = std::mem::replace(&mut self.illuminated_chars, chars_in_range);
    }
}

impl EffectHooks for Spotlights {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Spotlights {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let dynamic_neutral_gray = hex_color("#808080");
        self.spotlights = self.make_spotlights(ctx, self.config.spotlight_count)?;
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
            CharacterFilter::default(),
            CharacterSort::TopToBottomLeftToRight,
        );
        self.character_color_map.clear();
        self.character_color_map.resize(
            ctx.terminal.arena.len(),
            (ColorPair::default(), ColorPair::default()),
        );

        for &id in &characters {
            let (input_coord, input_fg, input_bg) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_coord,
                    ch.animation.input_fg_color.clone(),
                    ch.animation.input_bg_color.clone(),
                )
            };
            let (bright_pair, dark_pair) = if dynamic {
                if input_fg.is_some() || input_bg.is_some() {
                    let mut bright_fg = input_fg.clone();
                    if bright_fg.is_none() && input_bg.is_some() {
                        bright_fg = Some(dynamic_neutral_gray.clone());
                    }
                    let bp = ColorPair::new(bright_fg.clone(), input_bg.clone());
                    let dp = ColorPair::new(
                        bright_fg
                            .as_ref()
                            .map(|fg| Animation::adjust_color_brightness(fg, 0.2)),
                        input_bg
                            .as_ref()
                            .map(|bg| Animation::adjust_color_brightness(bg, 0.2)),
                    );
                    (bp, dp)
                } else {
                    let bp = ColorPair::new(Some(dynamic_neutral_gray.clone()), None);
                    let dp = ColorPair::new(
                        Some(Animation::adjust_color_brightness(
                            &dynamic_neutral_gray,
                            0.2,
                        )),
                        None,
                    );
                    (bp, dp)
                }
            } else {
                let color_bright = final_gradient_mapping
                    .get(&input_coord)
                    .cloned()
                    .unwrap_or_else(|| dynamic_neutral_gray.clone());
                let dp = ColorPair::new(
                    Some(Animation::adjust_color_brightness(&color_bright, 0.2)),
                    None,
                );
                let bp = ColorPair::new(Some(color_bright), None);
                (bp, dp)
            };
            ctx.terminal.set_character_visibility(id, true);
            let idx = id.0 as usize;
            if idx >= self.character_color_map.len() {
                self.character_color_map
                    .resize(idx + 1, (ColorPair::default(), ColorPair::default()));
            }
            self.character_color_map[idx] = (bright_pair, dark_pair.clone());
            let ch = &mut ctx.terminal.arena[idx];
            let input_symbol = ch.input_symbol.clone();
            let uses_pre = ch.uses_input_preexisting_colors;
            ch.animation.set_appearance(
                &input_symbol,
                uses_pre,
                Some(&input_symbol),
                Some(dark_pair),
            );
        }
        let smallest_dimension = std::cmp::min(ctx.terminal.canvas.right, ctx.terminal.canvas.top);
        self.illuminate_range = std::cmp::max(
            (smallest_dimension as f64 / self.config.beam_width_ratio)
                .floor()
                .min(smallest_dimension as f64) as i64,
            1,
        );
        self.search_duration = self.config.search_duration;
        self.searching = true;
        self.expanding = false;
        self.complete = false;
        for &spotlight in &self.spotlights.clone() {
            ctx.activate_path(self, spotlight, "0");
            ctx.active_characters.insert(spotlight);
        }
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.complete {
            self.illuminate_chars(ctx, self.illuminate_range);
            if self.searching {
                self.search_duration -= 1;
                if self.search_duration == 0 {
                    for &spotlight in &self.spotlights.clone() {
                        ctx.activate_path(self, spotlight, "center");
                    }
                    self.searching = false;
                }
            }
            if !self.spotlights.iter().any(|&s| {
                ctx.terminal.arena[s.0 as usize]
                    .motion
                    .active_path
                    .is_some()
            }) {
                while self.spotlights.len() > 1 {
                    self.spotlights.pop();
                }
                self.expanding = true;
                self.illuminate_range += 1;
                let limit = (std::cmp::max(ctx.terminal.canvas.right, ctx.terminal.canvas.top)
                    as f64
                    / 1.5)
                    .floor();
                if self.illuminate_range as f64 > limit {
                    self.complete = true;
                }
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}
