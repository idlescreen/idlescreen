//! sweep, ported from effects/effect_sweep.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_character_group, parse_gradient_direction,
    parse_gradient_steps, parse_symbol,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::{Easing, SequenceEaser};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod sweep_step;
#[allow(unused_imports)]
use sweep_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct SweepConfig {
    /// Space separated list of symbols to use for the sweep shimmer.
    #[arg(long = "sweep-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["█", "▓", "▒", "░"])]
    pub sweep_symbols: Vec<String>,

    /// Direction of the first sweep, revealing uncolored characters.
    #[arg(long = "first-sweep-direction", default_value = "column_right_to_left",
          value_parser = parse_character_group)]
    pub first_sweep_direction: CharacterGroup,

    /// Direction of the second sweep, coloring the characters.
    #[arg(long = "second-sweep-direction", default_value = "column_left_to_right",
          value_parser = parse_character_group)]
    pub second_sweep_direction: CharacterGroup,

    /// Space separated, unquoted, list of colors for the character gradient (applied from bottom to top).
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["8A008A", "00D1FF", "ffffff"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["8"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
pub struct Sweep {
    pub(super) config: SweepConfig,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) dynamic_second_sweep_palette: Vec<Color>,
    pub(super) complete: bool,
    pub(super) first_phase: bool,
    pub(super) easer: Option<SequenceEaser<Vec<CharId>>>,
    pub(super) groups_second_sweep: Vec<Vec<CharId>>,
}
impl Sweep {
    pub fn new(config: SweepConfig) -> Self {
        Sweep {
            config,
            character_final_color_map: HashMap::new(),
            dynamic_second_sweep_palette: Vec::new(),
            complete: false,
            first_phase: true,
            easer: None,
            groups_second_sweep: Vec::new(),
        }
    }
}
impl EffectHooks for Sweep {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
