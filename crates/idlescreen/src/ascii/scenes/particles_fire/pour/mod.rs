//! pour, ported from effects/effect_pour.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod pour_step;

/// PourIterator.PourDirection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PourDirection {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Debug, Clone)]
pub struct PourConfig {
    pub pour_direction: PourDirection,
    pub pour_speed: i64,
    pub movement_speed_range: (f64, f64),
    pub gap: i64,
    pub starting_color: Color,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_frames: i64,
    pub final_gradient_direction: GradientDirection,
    pub movement_easing: Easing,
}
impl Default for PourConfig {
    fn default() -> Self {
        Self {
            pour_direction: PourDirection::Down,
            pour_speed: 5,
            movement_speed_range: (0.8, 1.2),
            gap: 0,
            starting_color: hex_color("ffffff"),
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("FFFFFF"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_frames: 6,
            final_gradient_direction: GradientDirection::Vertical,
            movement_easing: Easing::InQuad,
        }
    }
}
pub struct Pour {
    pub(super) config: PourConfig,
    pub(super) pending_groups: Vec<Vec<CharId>>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) current_group: Vec<CharId>,
    pub(super) gap: i64,
}
impl Pour {
    pub fn new(config: PourConfig) -> Self {
        Pour {
            config,
            pending_groups: Vec::new(),
            character_final_color_map: HashMap::new(),
            current_group: Vec::new(),
            gap: 0,
        }
    }
}
impl EffectHooks for Pour {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
