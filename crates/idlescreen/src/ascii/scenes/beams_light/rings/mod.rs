//! rings, ported from effects/effect_rings.py.
//!
//! The inner RingsIterator.Ring class is the `Ring` struct; Python Path/Scene
//! object references become path/scene id strings (Path equality is by id).
//! No observable set iteration beyond the engine-canonical active_characters
//! (docs/ordering-inventory.md).

use std::collections::{HashMap, HashSet};

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
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

mod rings_build;
mod rings_sim;
mod rings_step;

/// Callback id: terminal.set_character_visibility(character, False).
pub(super) const CB_SET_INVISIBLE: u32 = 0;
#[derive(Debug, Clone)]
pub struct RingsConfig {
    pub ring_colors: Vec<Color>,
    pub ring_gap: f64,
    pub spin_duration: i64,
    pub spin_speed: (f64, f64),
    pub disperse_duration: i64,
    pub spin_disperse_cycles: i64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for RingsConfig {
    fn default() -> Self {
        Self {
            ring_colors: vec![
                hex_color("ab48ff"),
                hex_color("e7b2b2"),
                hex_color("fffebd"),
            ],
            ring_gap: 0.1,
            spin_duration: 90,
            spin_speed: (0.6, 1.8),
            disperse_duration: 60,
            spin_disperse_cycles: 2,
            final_gradient_stops: vec![
                hex_color("ab48ff"),
                hex_color("e7b2b2"),
                hex_color("fffebd"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
/// RingsIterator.Ring.
#[allow(clippy::struct_field_names)]
pub(super) struct Ring {
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
#[allow(clippy::struct_field_names)]
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
        ring_coords: Vec<Coord>,
        ring_gap: i64,
        ring_color: Color,
    ) -> Self {
        let clockwise_coords: Vec<Coord> = ring_coords.iter().rev().copied().collect();
        let rotation_speed = ctx.rng.uniform(config.spin_speed.0, config.spin_speed.1);
        Ring {
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
        let path = ch
            .motion
            .paths
            .get_mut(&path_id)
            .ok_or_else(|| EngineError::Other(String::new()))?;
        for coord in waypoint_coords {
            path.new_waypoint(coord, None, "")
                .map_err(EngineError::Other)?;
        }
        Ok(path_id)
    }
}

impl EffectHooks for Rings {
    fn dispatch_callback(
        &mut self,
        ctx: &mut EngineCtx,
        character: CharId,
        callback: &EffectCallback,
    ) {
        if callback.id == CB_SET_INVISIBLE {
            ctx.terminal.set_character_visibility(character, false);
        }
    }
}
