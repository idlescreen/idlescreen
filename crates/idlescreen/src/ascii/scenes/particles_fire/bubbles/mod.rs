//! bubbles, ported from effects/effect_bubbles.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_positive_float, parse_positive_int,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::{self, Coord};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod bubbles_step;
#[allow(unused_imports)]
use bubbles_step::*;
mod bubbles_sim;
#[allow(unused_imports)]
use bubbles_sim::*;

/// pop_condition choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopCondition {
    Row,
    Bottom,
    Anywhere,
}
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct BubblesConfig {
    /// If set, the bubbles will be colored with a rotating rainbow gradient.
    #[arg(long = "rainbow", default_value_t = false)]
    pub rainbow: bool,

    /// Space separated, unquoted, list of colors for the bubbles. Ignored if --no-rainbow is left as default False.
    #[arg(long = "bubble-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["d33aff", "7395c4", "43c2a7", "02ff7f"])]
    pub bubble_colors: Vec<Color>,

    /// Color for the spray emitted when a bubble pops.
    #[arg(long = "pop-color", default_value = "ffffff", value_parser = parse_color)]
    pub pop_color: Color,

    /// Speed of the floating bubbles.
    #[arg(long = "bubble-speed", default_value_t = 1.1, value_parser = parse_positive_float)]
    pub bubble_speed: f64,

    /// Number of frames between bubbles.
    #[arg(long = "bubble-delay", default_value_t = 4, value_parser = parse_positive_int)]
    pub bubble_delay: i64,

    /// Condition for a bubble to pop.
    #[arg(long = "pop-condition", default_value = "row", value_parser = parse_pop_condition)]
    pub pop_condition: PopCondition,

    /// Easing function to use for character movement after a bubble pops.
    #[arg(long = "movement-easing", default_value = "in_out_sine", value_parser = parse_easing)]
    pub movement_easing: Easing,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["d33aff", "02ff7f"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "diagonal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
/// BubblesIterator.Bubble state (methods live on Bubbles for hooks access).
pub(super) struct Bubble {
    pub(super) characters: Vec<CharId>,
    pub(super) radius: i64,
    pub(super) anchor_char: CharId,
    pub(super) lowest_row: i64,
    pub(super) landed: bool,
}
pub struct Bubbles {
    pub(super) config: BubblesConfig,
    pub(super) bubbles: Vec<Bubble>,
    pub(super) animating_bubbles: Vec<Bubble>,
    pub(super) rainbow_gradient: Gradient,
    pub(super) character_final_color_map: HashMap<CharId, Color>,
    pub(super) steps_since_last_bubble: i64,
}
pub(super) fn parse_pop_condition(s: &str) -> Result<PopCondition, String> {
    Ok(match s {
        "row" => PopCondition::Row,
        "bottom" => PopCondition::Bottom,
        "anywhere" => PopCondition::Anywhere,
        _ => {
            return Err(format!(
                "invalid choice: '{s}' (choose from 'row', 'bottom', 'anywhere')"
            ));
        }
    })
}
