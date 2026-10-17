//! rain, ported from effects/effect_rain.py.

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

mod rain_step;

#[derive(Debug, Clone)]
pub struct RainConfig {
    pub rain_colors: Vec<Color>,
    pub movement_speed: (f64, f64),
    pub rain_symbols: Vec<String>,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
    pub movement_easing: Easing,
}
impl Default for RainConfig {
    fn default() -> Self {
        Self {
            rain_colors: vec![
                hex_color("00315C"),
                hex_color("004C8F"),
                hex_color("0075DB"),
                hex_color("3F91D9"),
                hex_color("78B9F2"),
                hex_color("9AC8F5"),
                hex_color("B8D8F8"),
                hex_color("E3EFFC"),
            ],
            movement_speed: (0.6, 1.1),
            rain_symbols: vec![
                "o".to_string(),
                ".".to_string(),
                ",".to_string(),
                "*".to_string(),
                "|".to_string(),
            ],
            final_gradient_stops: vec![
                hex_color("488bff"),
                hex_color("b2e7de"),
                hex_color("57eaf7"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Diagonal,
            movement_easing: Easing::InExpo,
        }
    }
}
pub struct Rain {
    pub(super) config: RainConfig,
    pub(super) pending_chars: Vec<CharId>,
    pub(super) group_by_row: BTreeMap<i64, Vec<CharId>>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
}
impl Rain {
    pub fn new(config: RainConfig) -> Self {
        Rain {
            config,
            pending_chars: Vec::new(),
            group_by_row: BTreeMap::new(),
            character_final_color_map: HashMap::new(),
        }
    }
}
impl EffectHooks for Rain {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
