//! middleout, ported from effects/effect_middleout.py.
//!
//! Ordering note: upstream's __next__ iterates the freshly rebuilt
//! active_characters *set* (effect_middleout.py:229-232) to activate the
//! "full" path/scene. Canonical order here is ascending character_id
//! (docs/ordering-inventory.md), matched by a shim patch on
//! MiddleOutIterator.__next__.

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

/// typing.Literal["vertical", "horizontal"].
mod middleout_step;
#[allow(unused_imports)]
use middleout_step::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandDirection {
    Vertical,
    Horizontal,
}
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct MiddleoutConfig {
    /// Color for the initial text in the center of the canvas.
    #[arg(long = "starting-color", default_value = "ffffff", value_parser = parse_color)]
    pub starting_color: Color,

    /// Direction the text will expand.
    #[arg(long = "expand-direction", default_value = "vertical", value_parser = parse_expand_direction)]
    pub expand_direction: ExpandDirection,

    /// Speed of the characters during the initial expansion of the center vertical/horiztonal line.
    #[arg(long = "center-movement-speed", default_value_t = 1.1, value_parser = parse_positive_float)]
    pub center_movement_speed: f64,

    /// Speed of the characters during the final full expansion.
    #[arg(long = "full-movement-speed", default_value_t = 1.1, value_parser = parse_positive_float)]
    pub full_movement_speed: f64,

    /// Easing function to use for initial expansion.
    #[arg(long = "center-easing", default_value = "out_expo", value_parser = parse_easing)]
    pub center_easing: Easing,

    /// Easing function to use for full expansion.
    #[arg(long = "full-easing", default_value = "out_expo", value_parser = parse_easing)]
    pub full_easing: Easing,

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
    Center,
    Full,
}
pub struct Middleout {
    pub(super) config: MiddleoutConfig,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) phase: Phase,
}
pub(super) fn parse_expand_direction(s: &str) -> Result<ExpandDirection, String> {
    Ok(match s {
        "vertical" => ExpandDirection::Vertical,
        "horizontal" => ExpandDirection::Horizontal,
        _ => {
            return Err(format!(
                "invalid choice: '{s}' (choose from 'vertical', 'horizontal')"
            ));
        }
    })
}
impl Middleout {
    pub fn new(config: MiddleoutConfig) -> Self {
        Middleout {
            config,
            character_final_color_map: HashMap::new(),
            phase: Phase::Center,
        }
    }
}
impl EffectHooks for Middleout {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
