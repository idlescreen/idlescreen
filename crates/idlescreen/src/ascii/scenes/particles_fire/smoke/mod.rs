//! smoke, ported from effects/effect_smoke.py.
//!
//! RNG order mirrors SmokeIterator.__init__: PrimsWeighted construction
//! (random starting coord + one randint(0, 99) weight per character over
//! get_characters(inner+outer fill)), then the BreadthFirst starting-coord
//! draw, then build() (PrimsWeighted run to completion). __next__ consumes no
//! RNG. BreadthFirst's links-set traversal is the canonical ascending
//! character_id order (shim-patched; docs/ordering-inventory.md) — no new
//! observable set iterations in this effect.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::spanning_tree::{BreadthFirst, PrimsWeighted};

mod smoke_step;

#[derive(Debug, Clone)]
pub struct SmokeConfig {
    pub starting_color: Color,
    pub smoke_symbols: Vec<String>,
    pub smoke_gradient_stops: Vec<Color>,
    pub use_whole_canvas: bool,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for SmokeConfig {
    fn default() -> Self {
        Self {
            starting_color: hex_color("7A7A7A"),
            smoke_symbols: vec![
                "░".to_string(),
                "▒".to_string(),
                "▓".to_string(),
                "▒".to_string(),
                "░".to_string(),
            ],
            smoke_gradient_stops: vec![hex_color("242424"), hex_color("FFFFFF")],
            use_whole_canvas: false,
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
pub struct Smoke {
    pub(super) config: SmokeConfig,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    /// Option so next_frame can move it out of self while stepping needs ctx
    /// and event dispatch needs &mut self.
    pub(super) fill_alg: Option<BreadthFirst>,
}
impl Smoke {
    pub fn new(config: SmokeConfig) -> Self {
        Smoke {
            config,
            character_final_color_map: HashMap::new(),
            fill_alg: None,
        }
    }
}
impl EffectHooks for Smoke {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
