//! orbittingvolley, ported from effects/effect_orbittingvolley.py.
//!
//! The inner OrbittingVolleyIterator.Launcher class is the `Launcher` struct.
//! No observable set iteration beyond the engine-canonical active_characters
//! (docs/ordering-inventory.md); the effect consumes no RNG at all.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_int, parse_non_negative_ratio, parse_positive_float, parse_symbol,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::ExistingColorHandling;
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, CoordColorMap, Gradient, GradientDirection};

mod orbittingvolley_step;
#[allow(unused_imports)]
use orbittingvolley_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct OrbittingVolleyConfig {
    /// Symbol for the top launcher.
    #[arg(long = "top-launcher-symbol", default_value = "█", value_parser = parse_symbol)]
    pub top_launcher_symbol: String,

    /// Symbol for the right launcher.
    #[arg(long = "right-launcher-symbol", default_value = "█", value_parser = parse_symbol)]
    pub right_launcher_symbol: String,

    /// Symbol for the bottom launcher.
    #[arg(long = "bottom-launcher-symbol", default_value = "█", value_parser = parse_symbol)]
    pub bottom_launcher_symbol: String,

    /// Symbol for the left launcher.
    #[arg(long = "left-launcher-symbol", default_value = "█", value_parser = parse_symbol)]
    pub left_launcher_symbol: String,

    /// Orbitting speed of the launchers.
    #[arg(long = "launcher-movement-speed", default_value_t = 1.5, value_parser = parse_positive_float)]
    pub launcher_movement_speed: f64,

    /// Speed of the launched characters.
    #[arg(long = "character-movement-speed", default_value_t = 2.4, value_parser = parse_positive_float)]
    pub character_movement_speed: f64,

    /// Percent of total input characters each launcher will fire per volley. Lower limit of one character.
    #[arg(long = "volley-size", default_value_t = 0.08, value_parser = parse_non_negative_ratio)]
    pub volley_size: f64,

    /// Number of animation ticks to wait between volleys of characters.
    #[arg(long = "launch-delay", default_value_t = 10, value_parser = parse_non_negative_int)]
    pub launch_delay: i64,

    /// Easing function to use for launched character movement.
    #[arg(long = "character-easing", default_value = "out_sine", value_parser = parse_easing)]
    pub character_easing: Easing,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["FFA15C", "44D492"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "radial", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
/// OrbittingVolleyIterator.Launcher.
pub(super) struct Launcher {
    pub(super) character: CharId,
    pub(super) magazine: Vec<CharId>,
}
pub struct OrbittingVolley {
    pub(super) config: OrbittingVolleyConfig,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) launcher_gradient_coordinate_map: CoordColorMap,
    pub(super) final_gradient_last_color: Option<Color>,
    pub(super) launchers: Vec<Launcher>,
    pub(super) delay: i64,
    pub(super) complete: bool,
}
impl OrbittingVolley {
    pub fn new(config: OrbittingVolleyConfig) -> Self {
        OrbittingVolley {
            config,
            character_final_color_map: HashMap::new(),
            launcher_gradient_coordinate_map: CoordColorMap::default(),
            final_gradient_last_color: None,
            launchers: Vec::new(),
            delay: 0,
            complete: false,
        }
    }

    /// Launcher.build_paths (only called for the main launcher).
    pub(super) fn build_launcher_paths(
        &self,
        ctx: &mut EngineCtx,
        id: CharId,
    ) -> Result<(), EngineError> {
        let waypoints = [
            Coord::new(ctx.terminal.canvas.left, ctx.terminal.canvas.top),
            Coord::new(ctx.terminal.canvas.right, ctx.terminal.canvas.top),
        ];
        let input_coord = ctx.terminal.arena[id.0 as usize].input_coord;
        let waypoint_start_index = waypoints.iter().position(|&c| c == input_coord).must_val();
        let ch = &mut ctx.terminal.arena[id.0 as usize];
        let perimeter_path = ch
            .motion
            .new_path(
                self.config.launcher_movement_speed,
                None,
                Some(2),
                0,
                false,
                "perimeter",
            )
            .map_err(EngineError::Other)?;
        let path = ch.motion.paths.get_mut(&perimeter_path).must_val();
        for waypoint in waypoints[waypoint_start_index..]
            .iter()
            .chain(waypoints[..waypoint_start_index].iter())
        {
            path.new_waypoint(*waypoint, None, "")
                .map_err(EngineError::Other)?;
        }
        Ok(())
    }

    /// Launcher.launch.
    pub(super) fn launch(&mut self, ctx: &mut EngineCtx, launcher_index: usize) -> Option<CharId> {
        let launcher = &mut self.launchers[launcher_index];
        if launcher.magazine.is_empty() {
            return None;
        }
        let next_char = launcher.magazine.remove(0);
        let launcher_coord = ctx.terminal.arena[launcher.character.0 as usize]
            .motion
            .current_coord;
        ctx.terminal.arena[next_char.0 as usize]
            .motion
            .set_coordinate(launcher_coord);
        ctx.activate_path(self, next_char, "input_path");
        ctx.terminal.set_character_visibility(next_char, true);
        Some(next_char)
    }

    /// OrbittingVolleyIterator._set_launcher_coordinates.
    pub(super) fn set_launcher_coordinates(
        &mut self,
        ctx: &mut EngineCtx,
        parent_index: usize,
        child_index: usize,
    ) {
        let canvas_top = ctx.terminal.canvas.top;
        let canvas_bottom = ctx.terminal.canvas.bottom;
        let canvas_left = ctx.terminal.canvas.left;
        let canvas_right = ctx.terminal.canvas.right;
        let parent_char = self.launchers[parent_index].character;
        let child_char = self.launchers[child_index].character;
        let parent_progress = ctx.terminal.arena[parent_char.0 as usize]
            .motion
            .current_coord
            .column as f64
            / canvas_right as f64;
        let child_input_coord = ctx.terminal.arena[child_char.0 as usize].input_coord;
        if child_input_coord == Coord::new(canvas_right, canvas_top) {
            let child_row = canvas_top - (canvas_top as f64 * parent_progress) as i64;
            ctx.terminal.arena[child_char.0 as usize]
                .motion
                .set_coordinate(Coord::new(canvas_right, std::cmp::max(1, child_row)));
        } else if child_input_coord == Coord::new(canvas_right, canvas_bottom) {
            let child_column = canvas_right - (canvas_right as f64 * parent_progress) as i64;
            ctx.terminal.arena[child_char.0 as usize]
                .motion
                .set_coordinate(Coord::new(std::cmp::max(1, child_column), canvas_bottom));
        } else if child_input_coord == Coord::new(canvas_left, canvas_bottom) {
            let child_row = canvas_bottom + (canvas_top as f64 * parent_progress) as i64;
            ctx.terminal.arena[child_char.0 as usize]
                .motion
                .set_coordinate(Coord::new(
                    canvas_left,
                    std::cmp::min(canvas_top, child_row),
                ));
        }
        let current_coord = ctx.terminal.arena[child_char.0 as usize]
            .motion
            .current_coord;
        let color = self
            .launcher_gradient_coordinate_map
            .get(&current_coord)
            .must_val()
            .clone();
        let ch = &mut ctx.terminal.arena[child_char.0 as usize];
        let input_symbol = ch.input_symbol.clone();
        let uses_pre = ch.uses_input_preexisting_colors;
        ch.animation.set_appearance(
            &input_symbol,
            uses_pre,
            Some(&input_symbol.clone()),
            Some(ColorPair::new(Some(color), None)),
        );
    }
}
impl EffectHooks for OrbittingVolley {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
