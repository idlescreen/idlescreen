//! rings, ported from effects/effect_rings.py.
//!
//! The inner RingsIterator.Ring class is the `Ring` struct; Python Path/Scene
//! object references become path/scene id strings (Path equality is by id).
//! No observable set iteration beyond the engine-canonical active_characters
//! (docs/ordering-inventory.md).

use std::collections::{HashMap, HashSet};

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_positive_float, parse_positive_float_range, parse_positive_int,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::{self, Coord};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::round_half_even;

mod rings_step;
#[allow(unused_imports)]
use rings_step::*;
mod rings_sim;
#[allow(unused_imports)]
use rings_sim::*;
mod rings_build;
#[allow(unused_imports)]
use rings_build::*;

/// Callback id: terminal.set_character_visibility(character, False).
pub(super) const CB_SET_INVISIBLE: u32 = 0;
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct RingsConfig {
    /// Space separated, unquoted, list of colors for the rings.
    #[arg(long = "ring-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["ab48ff", "e7b2b2", "fffebd"])]
    pub ring_colors: Vec<Color>,

    /// Distance between rings as a percent of the smallest canvas dimension.
    #[arg(long = "ring-gap", default_value_t = 0.1, value_parser = parse_positive_float)]
    pub ring_gap: f64,

    /// Number of frames for each cycle of the spin phase.
    #[arg(long = "spin-duration", default_value_t = 90, value_parser = parse_positive_int)]
    pub spin_duration: i64,

    /// Range of speeds for the rotation of the rings. The speed is randomly selected from this range for each ring.
    #[arg(long = "spin-speed", default_value = "0.6-1.8", value_parser = parse_positive_float_range)]
    pub spin_speed: (f64, f64),

    /// Number of frames spent in the dispersed state between spinning cycles.
    #[arg(long = "disperse-duration", default_value_t = 60, value_parser = parse_positive_int)]
    pub disperse_duration: i64,

    /// Number of times the animation will cycles between spinning rings and dispersed characters.
    #[arg(long = "spin-disperse-cycles", default_value_t = 2, value_parser = parse_positive_int)]
    pub spin_disperse_cycles: i64,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["ab48ff", "e7b2b2", "fffebd"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
/// RingsIterator.Ring.
pub(super) struct Ring {
    #[allow(dead_code)]
    pub(super) radius: i64,
    #[allow(dead_code)]
    pub(super) origin: Coord,
    pub(super) counter_clockwise_coords: Vec<Coord>,
    pub(super) clockwise_coords: Vec<Coord>,
    pub(super) ring_gap: i64,
    pub(super) ring_color: Color,
    pub(super) characters: Vec<CharId>,
    /// Path ids (upstream holds Path objects; Path equality is by id).
    pub(super) character_last_ring_path: HashMap<CharId, String>,
    pub(super) rotation_speed: f64,
}
/// dict phase strings from RingsIterator.__next__.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Start,
    Disperse,
    Spin,
    Final,
    Complete,
}
pub struct Rings {
    pub(super) config: RingsConfig,
    pub(super) rings: Vec<Ring>,
    pub(super) non_ring_chars: Vec<CharId>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) phase: Phase,
    pub(super) initial_disperse_complete: bool,
    pub(super) spin_time_remaining: i64,
    pub(super) disperse_time_remaining: i64,
    pub(super) cycles_remaining: i64,
    pub(super) initial_phase_time_remaining: i64,
}
impl Ring {
    /// Ring.__init__ — consumes one uniform() for rotation_speed.
    pub(super) fn new(
        ctx: &mut EngineCtx,
        config: &RingsConfig,
        radius: i64,
        origin: Coord,
        ring_coords: Vec<Coord>,
        ring_gap: i64,
        ring_color: Color,
    ) -> Self {
        let clockwise_coords: Vec<Coord> = ring_coords.iter().rev().copied().collect();
        let rotation_speed = ctx.rng.uniform(config.spin_speed.0, config.spin_speed.1);
        Ring {
            radius,
            origin,
            counter_clockwise_coords: ring_coords,
            clockwise_coords,
            ring_gap,
            ring_color,
            characters: Vec::new(),
            character_last_ring_path: HashMap::new(),
            rotation_speed,
        }
    }

    /// Ring.make_disperse_waypoints.
    pub(super) fn make_disperse_waypoints(
        &self,
        ctx: &mut EngineCtx,
        id: CharId,
        origin: Coord,
    ) -> Result<String, EngineError> {
        let disperse_coords = geometry::find_coords_in_rect(origin, self.ring_gap);
        let mut waypoint_coords: Vec<Coord> = Vec::with_capacity(5);
        for _ in 0..5 {
            let index = ctx.rng.randrange(0, disperse_coords.len() as i64) as usize;
            waypoint_coords.push(disperse_coords[index]);
        }
        let ch = &mut ctx.terminal.arena[id.0 as usize];
        ch.motion.paths.remove("disperse");
        let path_id = ch
            .motion
            .new_path(0.14, None, None, 0, true, "disperse")
            .map_err(EngineError::Other)?;
        let path = ch.motion.paths.get_mut(&path_id).must_val();
        for coord in waypoint_coords {
            path.new_waypoint(coord, None, "")
                .map_err(EngineError::Other)?;
        }
        Ok(path_id)
    }
}
