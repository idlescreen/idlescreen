//! bouncyballs, ported from effects/effect_bouncyballs.py.

use std::collections::{BTreeMap, HashMap};

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_int, parse_positive_float, parse_symbol,
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

mod bouncyballs_step;
#[allow(unused_imports)]
use bouncyballs_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct BouncyBallsConfig {
    /// Space separated list of colors from which ball colors will be randomly selected.
    #[arg(long = "ball-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["d1f4a5", "96e2a4", "5acda9"])]
    pub ball_colors: Vec<Color>,

    /// Space separated list of symbols to use for the balls.
    #[arg(long = "ball-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["*", "o", "O", "0", "."])]
    pub ball_symbols: Vec<String>,

    /// Number of frames between ball drops, increase to reduce ball drop rate.
    #[arg(long = "ball-delay", default_value_t = 2, value_parser = parse_non_negative_int)]
    pub ball_delay: i64,

    /// Movement speed of the characters.
    #[arg(long = "movement-speed", default_value_t = 0.75, value_parser = parse_positive_float)]
    pub movement_speed: f64,

    /// Easing function to use for character movement.
    #[arg(long = "movement-easing", default_value = "out_bounce", value_parser = parse_easing)]
    pub movement_easing: Easing,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["f8ffae", "43c6ac"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "diagonal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
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
