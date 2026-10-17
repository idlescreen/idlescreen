//! bouncyballs, ported from effects/effect_bouncyballs.py.

use std::collections::{BTreeMap, HashMap};

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

mod bouncyballs_step;

#[derive(Debug, Clone)]
pub struct BouncyBallsConfig {
    pub ball_colors: Vec<Color>,
    pub ball_symbols: Vec<String>,
    pub ball_delay: i64,
    pub movement_speed: f64,
    pub movement_easing: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for BouncyBallsConfig {
    fn default() -> Self {
        Self {
            ball_colors: vec![
                hex_color("d1f4a5"),
                hex_color("96e2a4"),
                hex_color("5acda9"),
            ],
            ball_symbols: vec![
                "*".to_string(),
                "o".to_string(),
                "O".to_string(),
                "0".to_string(),
                ".".to_string(),
            ],
            ball_delay: 2,
            movement_speed: 0.75,
            movement_easing: Easing::OutBounce,
            final_gradient_stops: vec![hex_color("f8ffae"), hex_color("43c6ac")],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Diagonal,
        }
    }
}
pub struct BouncyBalls {
    pub(super) config: BouncyBallsConfig,
    pub(super) pending_chars: Vec<CharId>,
    pub(super) group_by_row: BTreeMap<i64, Vec<CharId>>,
    pub(super) character_final_color_map: HashMap<CharId, Color>,
    pub(super) ball_delay: i64,
}
impl BouncyBalls {
    pub fn new(config: BouncyBallsConfig) -> Self {
        BouncyBalls {
            config,
            pending_chars: Vec::new(),
            group_by_row: BTreeMap::new(),
            character_final_color_map: HashMap::new(),
            ball_delay: 0,
        }
    }
}
impl EffectHooks for BouncyBalls {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
