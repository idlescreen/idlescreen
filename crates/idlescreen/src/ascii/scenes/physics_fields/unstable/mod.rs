//! unstable, ported from effects/effect_unstable.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_positive_float,
};
use ttfx::cli::parse_color;
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

mod unstable_step;
#[allow(unused_imports)]
use unstable_step::*;
mod unstable_sim;
#[allow(unused_imports)]
use unstable_sim::*;
mod unstable_build;
#[allow(unused_imports)]
use unstable_build::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct UnstableConfig {
    /// Color transitioned to as the characters become unstable.
    #[arg(long = "unstable-color", default_value = "ff9200", value_parser = parse_color)]
    pub unstable_color: Color,

    /// Easing function to use for character movement during the explosion.
    #[arg(long = "explosion-ease", default_value = "out_expo", value_parser = parse_easing)]
    pub explosion_ease: Easing,

    /// Speed of characters during explosion.
    #[arg(long = "explosion-speed", default_value_t = 1.75, value_parser = parse_positive_float)]
    pub explosion_speed: f64,

    /// Easing function to use for character reassembly.
    #[arg(long = "reassembly-ease", default_value = "out_expo", value_parser = parse_easing)]
    pub reassembly_ease: Easing,

    /// Speed of characters during reassembly.
    #[arg(long = "reassembly-speed", default_value_t = 1.6, value_parser = parse_positive_float)]
    pub reassembly_speed: f64,

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Rumble,
    Explosion,
    Reassembly,
}
pub(super) const DYNAMIC_NEUTRAL_GRAY: &str = "808080";
pub struct Unstable {
    pub(super) config: UnstableConfig,
    pub(super) jumbled_coords: HashMap<CharId, Coord>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) character_start_color_map: HashMap<CharId, ColorPair>,
    pub(super) explosion_hold_time: i64,
    pub(super) phase: Phase,
    pub(super) max_rumble_steps: i64,
    pub(super) current_rumble_steps: i64,
    pub(super) rumble_mod_delay: i64,
}
impl Unstable {
    pub fn new(config: UnstableConfig) -> Self {
        Unstable {
            config,
            jumbled_coords: HashMap::new(),
            character_final_color_map: HashMap::new(),
            character_start_color_map: HashMap::new(),
            explosion_hold_time: 10,
            phase: Phase::Rumble,
            max_rumble_steps: 40,
            current_rumble_steps: 0,
            rumble_mod_delay: 6,
        }
    }
}
pub(super) fn neutral_gray() -> Color {
    Color::from_hex(DYNAMIC_NEUTRAL_GRAY).must_val()
}
impl EffectHooks for Unstable {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
