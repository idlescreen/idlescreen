//! pour, ported from effects/effect_pour.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_int, parse_positive_float_range, parse_positive_int,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod pour_step;
#[allow(unused_imports)]
use pour_step::*;

/// PourIterator.PourDirection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PourDirection {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct PourConfig {
    /// Direction the text will pour.
    #[arg(long = "pour-direction", default_value = "down", value_parser = parse_pour_direction)]
    pub pour_direction: PourDirection,

    /// Number of characters poured in per tick. Increase to speed up the effect.
    #[arg(long = "pour-speed", default_value_t = 5, value_parser = parse_positive_int)]
    pub pour_speed: i64,

    /// Movement speed range of the characters.
    #[arg(long = "movement-speed-range", default_value = "0.8-1.2", value_parser = parse_positive_float_range)]
    pub movement_speed_range: (f64, f64),

    /// Number of frames to wait between each character in the pour effect.
    #[arg(long = "gap", default_value_t = 0, value_parser = parse_non_negative_int)]
    pub gap: i64,

    /// Color of the characters before the gradient starts.
    #[arg(long = "starting-color", default_value = "ffffff", value_parser = parse_color)]
    pub starting_color: Color,

    /// Space separated, unquoted, list of colors for the character gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["8A008A", "00D1FF", "FFFFFF"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Number of frames to display each gradient step.
    #[arg(long = "final-gradient-frames", default_value_t = 6)]
    pub final_gradient_frames: i64,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,

    /// Easing function to use for character movement.
    #[arg(long = "movement-easing", default_value = "in_quad", value_parser = parse_easing)]
    pub movement_easing: Easing,
}
pub struct Pour {
    pub(super) config: PourConfig,
    pub(super) pending_groups: Vec<Vec<CharId>>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) current_group: Vec<CharId>,
    pub(super) gap: i64,
}
pub(super) fn parse_pour_direction(s: &str) -> Result<PourDirection, String> {
    Ok(match s {
        "up" => PourDirection::Up,
        "down" => PourDirection::Down,
        "left" => PourDirection::Left,
        "right" => PourDirection::Right,
        _ => {
            return Err(format!(
                "invalid choice: '{s}' (choose from 'up', 'down', 'left', 'right')"
            ));
        }
    })
}
impl Pour {
    pub fn new(config: PourConfig) -> Self {
        Pour {
            config,
            pending_groups: Vec::new(),
            character_final_color_map: HashMap::new(),
            current_group: Vec::new(),
            gap: 0,
        }
    }
}
impl EffectHooks for Pour {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
