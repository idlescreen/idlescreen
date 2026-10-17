//! spray, ported from effects/effect_spray.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::floor_div;

mod spray_step;

/// SprayIterator.SprayPosition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprayPosition {
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
    Nw,
    Center,
}
#[derive(Debug, Clone)]
pub struct SprayConfig {
    pub spray_position: SprayPosition,
    pub spray_volume: f64,
    pub movement_speed_range: (f64, f64),
    pub movement_easing: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for SprayConfig {
    fn default() -> Self {
        Self {
            spray_position: SprayPosition::E,
            spray_volume: 0.02,
            movement_speed_range: (1.0, 2.2),
            movement_easing: Easing::OutExpo,
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("FFFFFF"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
pub struct Spray {
    pub(super) config: SprayConfig,
    pub(super) pending_chars: Vec<CharId>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) volume: i64,
}
impl Spray {
    pub fn new(config: SprayConfig) -> Self {
        Spray {
            config,
            pending_chars: Vec::new(),
            character_final_color_map: HashMap::new(),
            volume: 1,
        }
    }
}
impl EffectHooks for Spray {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
