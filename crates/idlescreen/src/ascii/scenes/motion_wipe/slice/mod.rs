//! slice, ported from effects/effect_slice.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::ExistingColorHandling;
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod slice_step;

#[derive(Debug, Clone)]
pub struct SliceConfig {
    pub slice_direction: String,
    pub movement_speed: f64,
    pub movement_easing: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for SliceConfig {
    fn default() -> Self {
        Self {
            slice_direction: "vertical".to_string(),
            movement_speed: 0.25,
            movement_easing: Easing::InOutExpo,
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("FFFFFF"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Diagonal,
        }
    }
}
pub struct Slice {
    pub(super) config: SliceConfig,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
}
impl Slice {
    pub fn new(config: SliceConfig) -> Self {
        Slice {
            config,
            character_final_color_map: HashMap::new(),
        }
    }
}
impl EffectHooks for Slice {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
