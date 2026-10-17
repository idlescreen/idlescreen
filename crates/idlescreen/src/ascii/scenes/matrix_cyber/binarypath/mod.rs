//! binarypath, ported from effects/effect_binarypath.py.
//!
//! The inner _BinaryRepresentation class is the `BinaryRepresentation` struct.
//! No observable set iteration beyond the engine-canonical active_characters
//! (docs/ordering-inventory.md).

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
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

mod binarypath_sim;
mod binarypath_step;

#[derive(Debug, Clone)]
pub struct BinaryPathConfig {
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
    pub binary_colors: Vec<Color>,
    pub movement_speed: f64,
    pub active_binary_groups: f64,
}
impl Default for BinaryPathConfig {
    fn default() -> Self {
        Self {
            final_gradient_stops: vec![hex_color("00d500"), hex_color("007500")],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Radial,
            binary_colors: vec![
                hex_color("044E29"),
                hex_color("157e38"),
                hex_color("45bf55"),
                hex_color("95ed87"),
            ],
            movement_speed: 2.0,
            active_binary_groups: 0.20,
        }
    }
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
/// Orientation: `"col"` or `"row"`.
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
