//! waves, ported from effects/effect_waves.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_easing, parse_gradient_direction, parse_gradient_steps,
    parse_positive_int, parse_symbol,
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
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod waves_step;
#[allow(unused_imports)]
use waves_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct WavesConfig {
    /// Symbols to use for the wave animation.
    #[arg(long = "wave-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂", "▁"])]
    pub wave_symbols: Vec<String>,

    /// Space separated, unquoted, list of colors for the character gradient (applied across the canvas).
    #[arg(long = "wave-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["f0ff65", "ffb102", "31a0d4", "ffb102", "f0ff65"])]
    pub wave_gradient_stops: Vec<Color>,

    /// Space separated, unquoted, list of the number of gradient steps to use.
    #[arg(long = "wave-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["6"])]
    pub wave_gradient_steps: Vec<i64>,

    /// Number of waves to generate. n > 0.
    #[arg(long = "wave-count", default_value_t = 4, value_parser = parse_positive_int)]
    pub wave_count: i64,

    /// The number of frames for each step of the wave.
    #[arg(long = "wave-length", default_value_t = 1, value_parser = parse_positive_int)]
    pub wave_length: i64,

    /// Direction of the wave.
    #[arg(long = "wave-direction", default_value = "column_left_to_right", value_parser = parse_wave_direction)]
    pub wave_direction: CharacterGroup,

    /// Easing function to use for wave travel.
    #[arg(long = "wave-easing", default_value = "in_out_quad", value_parser = parse_easing)]
    pub wave_easing: Easing,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["ffb102", "31a0d4", "f0ff65"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "diagonal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
pub struct Waves {
    pub(super) config: WavesConfig,
    pub(super) pending_columns: Vec<Vec<CharId>>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
}
/// waves --wave-direction choices (a subset of CharacterGroup, upstream grouping_map).
pub(super) fn parse_wave_direction(s: &str) -> Result<CharacterGroup, String> {
    Ok(match s {
        "column_left_to_right" => CharacterGroup::ColumnLeftToRight,
        "column_right_to_left" => CharacterGroup::ColumnRightToLeft,
        "row_top_to_bottom" => CharacterGroup::RowTopToBottom,
        "row_bottom_to_top" => CharacterGroup::RowBottomToTop,
        "center_to_outside" => CharacterGroup::CenterToOutside,
        "outside_to_center" => CharacterGroup::OutsideToCenter,
        _ => return Err(format!("invalid wave direction: '{s}'")),
    })
}
impl Waves {
    pub fn new(config: WavesConfig) -> Self {
        Waves {
            config,
            pending_columns: Vec::new(),
            character_final_color_map: HashMap::new(),
        }
    }
}
impl EffectHooks for Waves {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
