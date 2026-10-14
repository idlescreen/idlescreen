//! binarypath, ported from effects/effect_binarypath.py.
//!
//! The inner _BinaryRepresentation class is the `BinaryRepresentation` struct.
//! No observable set iteration beyond the engine-canonical active_characters
//! (docs/ordering-inventory.md).

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_ratio, parse_positive_float,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{Animation, ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod binarypath_step;
#[allow(unused_imports)]
use binarypath_step::*;
mod binarypath_sim;
#[allow(unused_imports)]
use binarypath_sim::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct BinaryPathConfig {
    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["00d500", "007500"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "radial", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,

    /// Space separated, unquoted, list of colors for the binary characters. Character color is randomly assigned from this list.
    #[arg(long = "binary-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["044E29", "157e38", "45bf55", "95ed87"])]
    pub binary_colors: Vec<Color>,

    /// Speed of the binary groups as they travel around the terminal.
    #[arg(long = "movement-speed", default_value_t = 2.0, value_parser = parse_positive_float)]
    pub movement_speed: f64,

    /// Maximum number of binary groups that are active at any given time as a percentage of the total number of binary groups. Lower this to improve performance.
    #[arg(long = "active-binary-groups", default_value_t = 0.20, value_parser = parse_non_negative_ratio)]
    pub active_binary_groups: f64,
}
/// BinaryPathIterator._BinaryRepresentation.
pub(super) struct BinaryRepresentation {
    pub(super) character: CharId,
    pub(super) binary_characters: Vec<CharId>,
    pub(super) pending_binary_characters: Vec<CharId>,
    pub(super) input_coord: Coord,
    pub(super) is_active: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Travel,
    Wipe,
}
/// typing.Literal["col", "row"].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Orientation {
    Col,
    Row,
}
pub struct BinaryPath {
    pub(super) config: BinaryPathConfig,
    pub(super) pending_binary_representations: Vec<BinaryRepresentation>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) last_frame_provided: bool,
    pub(super) active_binary_reps: Vec<BinaryRepresentation>,
    pub(super) complete: bool,
    pub(super) phase: Phase,
    pub(super) final_wipe_chars: Vec<Vec<CharId>>,
    pub(super) max_active_binary_groups: i64,
}
impl BinaryRepresentation {
    /// _BinaryRepresentation._travel_complete.
    pub(super) fn travel_complete(&self, ctx: &EngineCtx) -> bool {
        self.binary_characters.iter().all(|&bin_char| {
            ctx.terminal.arena[bin_char.0 as usize].motion.current_coord == self.input_coord
        })
    }
}
impl BinaryPath {
    pub fn new(config: BinaryPathConfig) -> Self {
        BinaryPath {
            config,
            pending_binary_representations: Vec::new(),
            character_final_color_map: HashMap::new(),
            last_frame_provided: false,
            active_binary_reps: Vec::new(),
            complete: false,
            phase: Phase::Travel,
            final_wipe_chars: Vec::new(),
            max_active_binary_groups: 0,
        }
    }
}
impl EffectHooks for BinaryPath {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
