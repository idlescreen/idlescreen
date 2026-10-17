//! middleout, ported from effects/effect_middleout.py.
//!
//! Ordering note: upstream's __next__ iterates the freshly rebuilt
//! active_characters *set* (effect_middleout.py:229-232) to activate the
//! "full" path/scene. Canonical order here is ascending character_id
//! (docs/ordering-inventory.md), matched by a shim patch on
//! MiddleOutIterator.__next__.

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

mod middleout_step;

/// Expand direction: `"vertical"` or `"horizontal"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandDirection {
    Vertical,
    Horizontal,
}
#[derive(Debug, Clone)]
pub struct MiddleoutConfig {
    pub starting_color: Color,
    pub expand_direction: ExpandDirection,
    pub center_movement_speed: f64,
    pub full_movement_speed: f64,
    pub center_easing: Easing,
    pub full_easing: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for MiddleoutConfig {
    fn default() -> Self {
        Self {
            starting_color: hex_color("ffffff"),
            expand_direction: ExpandDirection::Vertical,
            center_movement_speed: 1.1,
            full_movement_speed: 1.1,
            center_easing: Easing::OutExpo,
            full_easing: Easing::OutExpo,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Center,
    Full,
}
pub struct Middleout {
    pub(super) config: MiddleoutConfig,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) phase: Phase,
}
impl Middleout {
    pub fn new(config: MiddleoutConfig) -> Self {
        Middleout {
            config,
            character_final_color_map: HashMap::new(),
            phase: Phase::Center,
        }
    }
}
impl EffectHooks for Middleout {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
