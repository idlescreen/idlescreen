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

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps, parse_symbol,
};
use ttfx::cli::parse_color;
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
#[allow(unused_imports)]
use smoke_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct SmokeConfig {
    /// Color of the text before being colorized by the smoke.
    #[arg(long = "starting-color", default_value = "7A7A7A", value_parser = parse_color)]
    pub starting_color: Color,

    /// Symbols to use for the smoke. Strings will be used in sequence to create an animation.
    #[arg(long = "smoke-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["░", "▒", "▓", "▒", "░"])]
    pub smoke_symbols: Vec<String>,

    /// Space separated, unquoted, list of colors for the smoke gradient.
    #[arg(long = "smoke-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["242424", "FFFFFF"])]
    pub smoke_gradient_stops: Vec<Color>,

    /// If True, the entire canvas will be flooded. Otherwise the effect is limited to the text boundary.
    #[arg(long = "use-whole-canvas", default_value_t = false)]
    pub use_whole_canvas: bool,

    /// Space separated, unquoted, list of colors for the character gradient (applied across the canvas).
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["8A008A", "00D1FF", "FFFFFF"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
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
