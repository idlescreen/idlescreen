//! colorshift, ported from effects/effect_colorshift.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps, parse_positive_int,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::geometry;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod colorshift_step;
#[allow(unused_imports)]
use colorshift_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct ColorShiftConfig {
    /// Space separated, unquoted, list of colors for the gradient.
    #[arg(long = "gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["e81416", "ffa500", "faeb36", "79c314", "487de7", "4b369d", "70369d"])]
    pub gradient_stops: Vec<Color>,

    /// Number of gradient steps to use. More steps will create a smoother gradient animation.
    #[arg(long = "gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub gradient_steps: Vec<i64>,

    /// Number of frames to display each gradient step. Increase to slow down the gradient animation.
    #[arg(long = "gradient-frames", default_value_t = 1, value_parser = parse_positive_int)]
    pub gradient_frames: i64,

    /// Do not display the gradient as a wave.
    #[arg(long = "no-travel")]
    pub no_travel: bool,

    /// Direction the gradient travels across the canvas.
    #[arg(long = "travel-direction", default_value = "radial", value_parser = parse_gradient_direction)]
    pub travel_direction: GradientDirection,

    /// Reverse the gradient travel direction.
    #[arg(long = "reverse-travel-direction")]
    pub reverse_travel_direction: bool,

    /// Do not loop the gradient.
    #[arg(long = "no-loop")]
    pub no_loop: bool,

    /// Number of times to cycle the gradient.
    #[arg(long = "cycles", default_value_t = 2, value_parser = parse_positive_int)]
    pub cycles: i64,

    /// Skip the final gradient.
    #[arg(long = "skip-final-gradient")]
    pub skip_final_gradient: bool,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["e81416", "ffa500", "faeb36", "79c314", "487de7", "4b369d", "70369d"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use for the final gradient.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
pub struct ColorShift {
    pub(super) config: ColorShiftConfig,
    pub(super) character_final_color_map: HashMap<CharId, Color>,
    pub(super) loop_tracker_map: HashMap<CharId, i64>,
}
impl ColorShift {
    pub fn new(config: ColorShiftConfig) -> Self {
        ColorShift {
            config,
            character_final_color_map: HashMap::new(),
            loop_tracker_map: HashMap::new(),
        }
    }
}
impl EffectHooks for ColorShift {
    /// ColorShiftIterator.loop_tracker.
    fn dispatch_callback(
        &mut self,
        ctx: &mut EngineCtx,
        character: CharId,
        _callback: &EffectCallback,
    ) {
        let count = {
            let entry = self.loop_tracker_map.entry(character).or_insert(0);
            *entry += 1;
            *entry
        };
        if self.config.cycles == 0 || count < self.config.cycles {
            ctx.activate_scene(self, character, "gradient");
        } else if !self.config.skip_final_gradient {
            ctx.activate_scene(self, character, "final_gradient");
        }
    }
}
