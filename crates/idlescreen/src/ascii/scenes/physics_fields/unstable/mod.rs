//! unstable, ported from effects/effect_unstable.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod unstable_build;
mod unstable_sim;
mod unstable_step;
use unstable_sim::uses_pre_of;

#[derive(Debug, Clone)]
pub struct UnstableConfig {
    pub unstable_color: Color,
    pub explosion_ease: Easing,
    pub explosion_speed: f64,
    pub reassembly_ease: Easing,
    pub reassembly_speed: f64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Rumble,
    Explosion,
    Reassembly,
}
pub(super) const DYNAMIC_NEUTRAL_GRAY: &str = "808080";
pub struct Unstable {
    pub(super) config: UnstableConfig,
    pub(super) jumbled_coords: HashMap<CharId, Coord>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) character_start_color_map: HashMap<CharId, ColorPair>,
    pub(super) explosion_hold_time: i64,
    pub(super) phase: Phase,
    pub(super) max_rumble_steps: i64,
    pub(super) current_rumble_steps: i64,
    pub(super) rumble_mod_delay: i64,
}
impl Unstable {
    pub fn new(config: UnstableConfig) -> Self {
        Unstable {
            config,
            jumbled_coords: HashMap::new(),
            character_final_color_map: HashMap::new(),
            character_start_color_map: HashMap::new(),
            explosion_hold_time: 10,
            phase: Phase::Rumble,
            max_rumble_steps: 40,
            current_rumble_steps: 0,
            rumble_mod_delay: 6,
        }
    }
}
pub(super) fn neutral_gray() -> Color {
    hex_color(DYNAMIC_NEUTRAL_GRAY)
}
impl EffectHooks for Unstable {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
