//! spray, ported from effects/effect_spray.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_positive_float_range, parse_positive_ratio,
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
use ttfx::utils::pycompat::floor_div;

mod spray_step;
#[allow(unused_imports)]
use spray_step::*;

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
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct SprayConfig {
    /// Position for the spray origin.
    #[arg(long = "spray-position", default_value = "e", value_parser = parse_spray_position)]
    pub spray_position: SprayPosition,

    /// Number of characters to spray per tick as a percent of the total number of characters.
    #[arg(long = "spray-volume", default_value_t = 0.02, value_parser = parse_positive_ratio)]
    pub spray_volume: f64,

    /// Movement speed range of the characters.
    #[arg(long = "movement-speed-range", default_value = "1.0-2.2", value_parser = parse_positive_float_range)]
    pub movement_speed_range: (f64, f64),

    /// Easing function to use for character movement.
    #[arg(long = "movement-easing", default_value = "out_expo", value_parser = parse_easing)]
    pub movement_easing: Easing,

    /// Space separated, unquoted, list of colors for the final color gradient.
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
pub struct Spray {
    pub(super) config: SprayConfig,
    pub(super) pending_chars: Vec<CharId>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) volume: i64,
}
pub(super) fn parse_spray_position(s: &str) -> Result<SprayPosition, String> {
    Ok(match s {
        "n" => SprayPosition::N,
        "ne" => SprayPosition::Ne,
        "e" => SprayPosition::E,
        "se" => SprayPosition::Se,
        "s" => SprayPosition::S,
        "sw" => SprayPosition::Sw,
        "w" => SprayPosition::W,
        "nw" => SprayPosition::Nw,
        "center" => SprayPosition::Center,
        _ => {
            return Err(format!(
                "invalid choice: '{s}' (choose from 'n', 'ne', 'e', 'se', 's', 'sw', 'w', 'nw', 'center')"
            ));
        }
    })
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
