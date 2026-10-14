//! slice, ported from effects/effect_slice.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_positive_float,
};
use ttfx::cli::parse_color;
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
#[allow(unused_imports)]
use slice_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct SliceConfig {
    /// Direction of the slice.
    #[arg(long = "slice-direction", default_value = "vertical",
          value_parser = ["vertical", "horizontal", "diagonal"])]
    pub slice_direction: String,

    /// Movement speed of the characters.
    #[arg(long = "movement-speed", default_value_t = 0.25, value_parser = parse_positive_float)]
    pub movement_speed: f64,

    /// Easing function to use for character movement.
    #[arg(long = "movement-easing", default_value = "in_out_expo", value_parser = parse_easing)]
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
    #[arg(long = "final-gradient-direction", default_value = "diagonal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
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
