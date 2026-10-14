//! fireworks, ported from effects/effect_fireworks.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_int, parse_non_negative_ratio, parse_symbol,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, SyncMetric, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::{self, Coord};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::{floor_div, round_half_even};

mod fireworks_step;
#[allow(unused_imports)]
use fireworks_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct FireworksConfig {
    /// If set, fireworks explode anywhere in the canvas. Otherwise, fireworks explode above highest settled row of text.
    #[arg(long = "explode-anywhere", default_value_t = false)]
    pub explode_anywhere: bool,

    /// Space separated list of colors from which firework colors will be randomly selected.
    #[arg(long = "firework-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["88F7E2", "44D492", "F5EB67", "FFA15C", "FA233E"])]
    pub firework_colors: Vec<Color>,

    /// Symbol to use for the firework shell.
    #[arg(long = "firework-symbol", default_value = "o", value_parser = parse_symbol)]
    pub firework_symbol: String,

    /// Percent of total characters in each firework shell.
    #[arg(long = "firework-volume", default_value_t = 0.08, value_parser = parse_non_negative_ratio)]
    pub firework_volume: f64,

    /// Number of frames to wait between launching each firework shell. +/- 0-50 percent randomness is applied to this value.
    #[arg(long = "launch-delay", default_value_t = 15, value_parser = parse_non_negative_int)]
    pub launch_delay: i64,

    /// Maximum distance from the firework shell origin to the explode waypoint as a percentage of the total canvas width.
    #[arg(long = "explode-distance", default_value_t = 0.2, value_parser = parse_non_negative_ratio)]
    pub explode_distance: f64,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["8A008A", "00D1FF", "FFFFFF"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "horizontal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
pub struct Fireworks {
    pub(super) config: FireworksConfig,
    pub(super) shells: Vec<Vec<CharId>>,
    pub(super) firework_volume: i64,
    pub(super) explode_distance: i64,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) launch_delay: i64,
}
impl Fireworks {
    pub fn new(config: FireworksConfig) -> Self {
        Fireworks {
            config,
            shells: Vec::new(),
            firework_volume: 0,
            explode_distance: 0,
            character_final_color_map: HashMap::new(),
            launch_delay: 0,
        }
    }

    /// FireworksIterator.prepare_waypoints.
    pub(super) fn prepare_waypoints(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let mut firework_shell: Vec<CharId> = Vec::new();
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        let canvas_bottom = ctx.terminal.canvas.bottom;
        let canvas_top = ctx.terminal.canvas.top;
        let canvas_right = ctx.terminal.canvas.right;
        // Loop variables assigned at each shell boundary and reused across the
        // following characters, like the Python loop-scoped names.
        let mut origin_x: i64 = 0;
        let mut origin_coord = Coord::new(0, 0);
        let mut explode_waypoint_coords: Vec<Coord> = Vec::new();
        for id in characters {
            if firework_shell.len() as i64 == self.firework_volume || firework_shell.is_empty() {
                origin_x = ctx.rng.randrange(0, canvas_right);
                self.shells.push(std::mem::take(&mut firework_shell));
                let min_row = if !self.config.explode_anywhere {
                    ctx.terminal.arena[id.0 as usize].input_coord.row
                } else {
                    canvas_bottom
                };
                let origin_y = ctx.rng.randrange(min_row, canvas_top + 1);
                origin_coord = Coord::new(origin_x, origin_y);
                explode_waypoint_coords =
                    geometry::find_coords_in_circle(origin_coord, self.explode_distance);
            }
            let input_coord = ctx.terminal.arena[id.0 as usize].input_coord;
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .set_coordinate(Coord::new(origin_x, canvas_bottom));
                let apex_path = ch
                    .motion
                    .new_path(0.65, Some(Easing::OutExpo), Some(2), 0, false, "apex_pth")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&apex_path)
                    .must_val()
                    .new_waypoint(origin_coord, None, "")
                    .map_err(EngineError::Other)?;
            }
            let apex_wpt_coord = origin_coord;
            let explode_speed = ctx.rng.uniform(0.5, 0.9);
            let explode_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .new_path(explode_speed, Some(Easing::OutCirc), Some(2), 0, false, "")
                    .map_err(EngineError::Other)?
            };
            let explode_wpt_coord = *ctx.rng.choice(&explode_waypoint_coords);
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .paths
                    .get_mut(&explode_path)
                    .must_val()
                    .new_waypoint(explode_wpt_coord, None, "")
                    .map_err(EngineError::Other)?;
            }

            let bloom_control_point = geometry::extrapolate_along_ray(
                apex_wpt_coord,
                explode_wpt_coord,
                floor_div(self.explode_distance, 2) as f64,
            );
            let bloom_wpt_coord = Coord::new(
                bloom_control_point.column,
                std::cmp::max(1, bloom_control_point.row - 7),
            );
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .paths
                    .get_mut(&explode_path)
                    .must_val()
                    .new_waypoint(bloom_wpt_coord, Some(vec![bloom_control_point]), "")
                    .map_err(EngineError::Other)?;
            }
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let input_path = ch
                    .motion
                    .new_path(
                        0.9,
                        Some(Easing::InOutQuart),
                        Some(2),
                        0,
                        false,
                        "input_pth",
                    )
                    .map_err(EngineError::Other)?;
                let input_control_point = Coord::new(bloom_wpt_coord.column, 1);
                ch.motion
                    .paths
                    .get_mut(&input_path)
                    .must_val()
                    .new_waypoint(input_coord, Some(vec![input_control_point]), "")
                    .map_err(EngineError::Other)?;
            }
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path("apex_pth".to_string()),
                EventAction::ActivatePath(explode_path.clone()),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(explode_path),
                EventAction::ActivatePath("input_pth".to_string()),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path("input_pth".to_string()),
                EventAction::SetLayer(0),
            )
            .map_err(EngineError::Other)?;

            ctx.activate_path(self, id, "apex_pth");

            firework_shell.push(id);
        }
        if !firework_shell.is_empty() {
            self.shells.push(firework_shell);
        }
        Ok(())
    }
}
