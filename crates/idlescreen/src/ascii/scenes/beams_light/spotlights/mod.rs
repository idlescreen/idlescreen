// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Spotlights effect, ported from effects/effect_spotlights.py with flat Vec colour lookup.

mod spotlights_step;

use clap::Parser;
use ttfx::cli::parse_color;
use ttfx::engine::active_characters::ActiveCharacters;
use ttfx::engine::animation::{Animation, ExistingColorHandling};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::EngineCtx;
use ttfx::engine::error::EngineError;
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::{self, Coord};
use ttfx::utils::graphics::{Color, ColorPair, GradientDirection};
use ttfx::utils::pycompat::floor_div;

use crate::ascii::scenes::canvas_ops::{
    parse_gradient_direction, parse_gradient_steps, parse_non_negative_float, parse_positive_float,
    parse_positive_float_range, parse_positive_int,
};

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct SpotlightsConfig {
    /// Width of the beam of light as min(width, height) // n of the input text. Values less than 1 are raised to 1.
    #[arg(long = "beam-width-ratio", default_value_t = 2.0, value_parser = parse_positive_float)]
    pub beam_width_ratio: f64,

    /// Distance from the edge of the beam where the brightness begins to fall off, as a percentage of total beam width.
    #[arg(long = "beam-falloff", default_value_t = 0.3, value_parser = parse_non_negative_float)]
    pub beam_falloff: f64,

    /// Duration of the search phase, in frames, before the spotlights converge in the center.
    #[arg(long = "search-duration", default_value_t = 160, value_parser = parse_positive_int)]
    pub search_duration: i64,

    /// Range of speeds for the spotlights during the search phase.
    #[arg(long = "search-speed-range", default_value = "0.85-1.60", value_parser = parse_positive_float_range)]
    pub search_speed_range: (f64, f64),

    /// Number of spotlights to use.
    #[arg(long = "spotlight-count", default_value_t = 4, value_parser = parse_positive_int)]
    pub spotlight_count: i64,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["00f5d4", "51a2da", "ffbe0b"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}

pub struct Spotlights {
    pub(super) config: SpotlightsConfig,
    pub(super) illuminated_chars: ActiveCharacters,
    pub(super) illuminated_scratch: ActiveCharacters,
    pub(super) character_color_map: Vec<(ColorPair, ColorPair)>,
    pub(super) spotlights: Vec<CharId>,
    pub(super) illuminate_range: i64,
    pub(super) search_duration: i64,
    pub(super) searching: bool,
    pub(super) expanding: bool,
    pub(super) complete: bool,
}

impl Spotlights {
    pub fn new(config: SpotlightsConfig) -> Self {
        Spotlights {
            config,
            illuminated_chars: ActiveCharacters::new(),
            illuminated_scratch: ActiveCharacters::new(),
            character_color_map: Vec::new(),
            spotlights: Vec::new(),
            illuminate_range: 1,
            search_duration: 0,
            searching: true,
            expanding: false,
            complete: false,
        }
    }

    pub(super) fn adjust_color_pair_brightness(
        colors: &ColorPair,
        brightness_factor: f64,
    ) -> ColorPair {
        ColorPair::new(
            colors
                .fg_color
                .as_ref()
                .map(|fg| Animation::adjust_color_brightness(fg, brightness_factor)),
            colors
                .bg_color
                .as_ref()
                .map(|bg| Animation::adjust_color_brightness(bg, brightness_factor)),
        )
    }

    pub(super) fn has_input_colors(ctx: &EngineCtx, id: CharId) -> bool {
        let ch = &ctx.terminal.arena[id.0 as usize];
        ch.animation.input_fg_color.is_some() || ch.animation.input_bg_color.is_some()
    }

    pub(super) fn is_spotlightable(ctx: &EngineCtx, id: CharId) -> bool {
        ctx.terminal.arena[id.0 as usize].input_symbol != " " || Self::has_input_colors(ctx, id)
    }

    pub(super) fn get_expand_color_override(
        &self,
        ctx: &EngineCtx,
        id: CharId,
    ) -> Option<ColorPair> {
        if ctx.terminal.config.existing_color_handling != ExistingColorHandling::Dynamic
            || !self.expanding
        {
            return None;
        }
        let ch = &ctx.terminal.arena[id.0 as usize];
        if ch.animation.input_fg_color.is_none() && ch.animation.input_bg_color.is_some() {
            return Some(ColorPair::new(None, ch.animation.input_bg_color.clone()));
        }
        if !Self::has_input_colors(ctx, id) {
            return Some(ColorPair::default());
        }
        None
    }

    pub(super) fn make_spotlights(
        &mut self,
        ctx: &mut EngineCtx,
        num_spotlights: i64,
    ) -> Result<Vec<CharId>, EngineError> {
        let mut spotlights: Vec<CharId> = Vec::new();
        let minimum_distance = floor_div(ctx.terminal.canvas.right, 4);
        for _ in 0..num_spotlights {
            let spawn_coord = ctx.terminal.canvas.random_coord(&mut ctx.rng, true, false);
            let spotlight = ctx.terminal.add_character("O", spawn_coord);
            spotlights.push(spotlight);

            let mut spotlight_target_coords: Vec<Coord> = Vec::new();
            let mut last_coord = ctx.terminal.canvas.random_coord(&mut ctx.rng, false, false);
            spotlight_target_coords.push(last_coord);
            for _ in 0..10 {
                let next_coord =
                    Self::find_coord_at_minimum_distance(ctx, last_coord, minimum_distance);
                spotlight_target_coords.push(next_coord);
                last_coord = next_coord;
            }

            let mut paths: Vec<String> = Vec::new();
            for coord in spotlight_target_coords {
                let speed = ctx.rng.uniform(
                    self.config.search_speed_range.0,
                    self.config.search_speed_range.1,
                );
                let path_id = paths.len().to_string();
                let path_id = {
                    let ch = &mut ctx.terminal.arena[spotlight.0 as usize];
                    ch.motion
                        .new_path(speed, Some(Easing::InOutQuad), None, 0, false, &path_id)
                        .map_err(EngineError::Other)?
                };
                let bezier_control = ctx.terminal.canvas.random_coord(&mut ctx.rng, true, false);
                if let Some(p) = ctx.terminal.arena[spotlight.0 as usize]
                    .motion
                    .paths
                    .get_mut(&path_id)
                {
                    p.new_waypoint(coord, Some(vec![bezier_control]), "")
                        .map_err(EngineError::Other)?;
                }
                paths.push(path_id);
            }
            ctx.chain_paths(spotlight, &paths, true)
                .map_err(EngineError::Other)?;

            let canvas_center = ctx.terminal.canvas.center;
            let ch = &mut ctx.terminal.arena[spotlight.0 as usize];
            let center_path = ch
                .motion
                .new_path(0.5, Some(Easing::InOutSine), None, 0, false, "center")
                .map_err(EngineError::Other)?;
            if let Some(p) = ch.motion.paths.get_mut(&center_path) {
                p.new_waypoint(canvas_center, None, "")
                    .map_err(EngineError::Other)?;
            }
        }
        Ok(spotlights)
    }

    fn find_coord_at_minimum_distance(
        ctx: &mut EngineCtx,
        origin_coord: Coord,
        minimum_distance: i64,
    ) -> Coord {
        loop {
            let coord = ctx.terminal.canvas.random_coord(&mut ctx.rng, false, false);
            let distance = geometry::find_length_of_line(origin_coord, coord, false);
            if distance >= minimum_distance as f64 {
                return coord;
            }
        }
    }
}

impl Default for SpotlightsConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}

impl Default for Spotlights {
    fn default() -> Self {
        Self::new(SpotlightsConfig::default())
    }
}
