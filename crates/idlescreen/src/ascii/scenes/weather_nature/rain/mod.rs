//! rain, ported from effects/effect_rain.py.

use std::collections::{BTreeMap, HashMap};

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_positive_float_range, parse_symbol,
};
use ttfx::cli::parse_color;
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
#[allow(unused_imports)]
use rain_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct RainConfig {
    /// List of colors for the rain drops. Colors are randomly chosen from the list.
    #[arg(long = "rain-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["00315C", "004C8F", "0075DB", "3F91D9", "78B9F2", "9AC8F5", "B8D8F8", "E3EFFC"])]
    pub rain_colors: Vec<Color>,

    /// Falling speed range of the rain drops.
    #[arg(long = "movement-speed", default_value = "0.6-1.1", value_parser = parse_positive_float_range)]
    pub movement_speed: (f64, f64),

    /// Space separated list of symbols to use for the rain drops. Symbols are randomly chosen from the list.
    #[arg(long = "rain-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["o", ".", ",", "*", "|"])]
    pub rain_symbols: Vec<String>,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["488bff", "b2e7de", "57eaf7"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "diagonal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,

    /// Easing function to use for character movement.
    #[arg(long = "movement-easing", default_value = "in_expo", value_parser = parse_easing)]
    pub movement_easing: Easing,
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
