//! errorcorrect, ported from effects/effect_errorcorrect.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_positive_float, parse_positive_int,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, SyncMetric, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod errorcorrect_step;
#[allow(unused_imports)]
use errorcorrect_step::*;
mod errorcorrect_sim;
#[allow(unused_imports)]
use errorcorrect_sim::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct ErrorCorrectConfig {
    /// Percent of characters that are in the wrong position. This is a float between 0 and 1.0.
    #[arg(long = "error-pairs", default_value_t = 0.1, value_parser = parse_positive_float)]
    pub error_pairs: f64,

    /// Number of frames between swaps.
    #[arg(long = "swap-delay", default_value_t = 3, value_parser = parse_positive_int)]
    pub swap_delay: i64,

    /// Color for the characters that are in the wrong position.
    #[arg(long = "error-color", default_value = "e74c3c", value_parser = parse_color)]
    pub error_color: Color,

    /// Color for the characters once corrected, this is a gradient from error-color and fades to final-color.
    #[arg(long = "correct-color", default_value = "45bf55", value_parser = parse_color)]
    pub correct_color: Color,

    /// Speed of the characters while moving to the correct position.
    #[arg(long = "movement-speed", default_value_t = 1.5, value_parser = parse_positive_float)]
    pub movement_speed: f64,

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
pub(super) const BLOCK_WIPE_START: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
pub(super) const BLOCK_WIPE_END: [&str; 7] = ["▇", "▆", "▅", "▄", "▃", "▂", "▁"];
pub struct ErrorCorrect {
    pub(super) config: ErrorCorrectConfig,
    pub(super) swapped: Vec<(CharId, CharId)>,
    pub(super) swap_delay: i64,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
}
impl ErrorCorrect {
    pub fn new(config: ErrorCorrectConfig) -> Self {
        ErrorCorrect {
            config,
            swapped: Vec::new(),
            swap_delay: 0,
            character_final_color_map: HashMap::new(),
        }
    }

    /// ErrorCorrectIterator._get_dynamic_final_scene.
    pub(super) fn get_dynamic_final_scene(
        &self,
        ctx: &mut EngineCtx,
        id: CharId,
    ) -> Result<String, EngineError> {
        let (input_symbol, input_fg, input_bg, uses_pre) = {
            let ch = &ctx.terminal.arena[id.0 as usize];
            (
                ch.input_symbol.clone(),
                ch.animation.input_fg_color.clone(),
                ch.animation.input_bg_color.clone(),
                ch.uses_input_preexisting_colors,
            )
        };
        let final_scene = {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation.new_scene(false, None, None, "", uses_pre)
        };
        let fg_gradient = match &input_fg {
            Some(fg) => Some(
                Gradient::with_steps(&[self.config.correct_color.clone(), fg.clone()], 10, false)
                    .map_err(EngineError::Other)?,
            ),
            None => None,
        };
        let bg_gradient = match &input_bg {
            Some(bg) => Some(
                Gradient::with_steps(&[self.config.correct_color.clone(), bg.clone()], 10, false)
                    .map_err(EngineError::Other)?,
            ),
            None => None,
        };
        let ch = &mut ctx.terminal.arena[id.0 as usize];
        let scene = ch.animation.scenes.get_mut(&final_scene).must_val();
        if fg_gradient.is_some() || bg_gradient.is_some() {
            scene
                .apply_gradient_to_symbols(
                    &[input_symbol.clone()],
                    3,
                    fg_gradient.as_ref(),
                    bg_gradient.as_ref(),
                )
                .map_err(EngineError::Other)?;
        } else {
            scene
                .add_frame(
                    &input_symbol,
                    3,
                    VisualParams {
                        colors: Some(ColorPair::default()),
                        ..Default::default()
                    },
                )
                .map_err(EngineError::Other)?;
        }
        Ok(final_scene)
    }
}
