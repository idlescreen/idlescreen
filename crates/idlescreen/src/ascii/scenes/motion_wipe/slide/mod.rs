//! slide, ported from effects/effect_slide.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_int, parse_positive_float,
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

mod slide_step;
#[allow(unused_imports)]
use slide_step::*;

/// typing.Literal["row", "column", "diagonal"].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlideGrouping {
    Row,
    Column,
    Diagonal,
}
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct SlideConfig {
    /// Speed of the characters.
    #[arg(long = "movement-speed", default_value_t = 0.8, value_parser = parse_positive_float)]
    pub movement_speed: f64,

    /// Direction to group characters.
    #[arg(long = "grouping", default_value = "row", value_parser = parse_slide_grouping)]
    pub grouping: SlideGrouping,

    /// Number of frames to wait before adding the next group of characters.
    #[arg(long = "gap", default_value_t = 2, value_parser = parse_non_negative_int)]
    pub gap: i64,

    /// Reverse the direction of the characters.
    #[arg(long = "reverse-direction")]
    pub reverse_direction: bool,

    /// Merge the character groups originating from either side of the terminal.
    #[arg(long = "merge")]
    pub merge: bool,

    /// Easing function to use for character movement.
    #[arg(long = "movement-easing", default_value = "in_out_quad", value_parser = parse_easing)]
    pub movement_easing: Easing,

    /// Space separated, unquoted, list of colors for the character gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["833ab4", "fd1d1d", "fcb045"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Number of frames to display each gradient step.
    #[arg(long = "final-gradient-frames", default_value_t = 6)]
    pub final_gradient_frames: i64,

    /// Direction of the gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
pub struct Slide {
    pub(super) config: SlideConfig,
    pub(super) pending_groups: Vec<Vec<CharId>>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) active_groups: Vec<Vec<CharId>>,
    pub(super) current_gap: i64,
}
pub(super) fn parse_slide_grouping(s: &str) -> Result<SlideGrouping, String> {
    Ok(match s {
        "row" => SlideGrouping::Row,
        "column" => SlideGrouping::Column,
        "diagonal" => SlideGrouping::Diagonal,
        _ => {
            return Err(format!(
                "invalid choice: '{s}' (choose from 'row', 'column', 'diagonal')"
            ));
        }
    })
}
impl Slide {
    pub fn new(config: SlideConfig) -> Self {
        Slide {
            config,
            pending_groups: Vec::new(),
            character_final_color_map: HashMap::new(),
            active_groups: Vec::new(),
            current_gap: 0,
        }
    }
}
impl EffectHooks for Slide {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
