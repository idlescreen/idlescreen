//! matrix, ported from effects/effect_matrix.py.
//!
//! THE CLOCK EFFECT: upstream reads time.time() at effect_matrix.py:430
//! (rain_start, set after build) and :549 (rain-phase deadline check). Both
//! sites use ctx.clock.now_wall(); the parity harness virtualizes the clock
//! on both sides (plan.md §4.7).
//!
//! The inner RainColumn class is the `RainColumn` struct. Columns migrate
//! between pending/active/full lists and are compared by identity upstream
//! (`column not in self.full_columns`), so ttfx keeps them in a Vec arena and
//! the lists hold indices. No observable set iteration beyond the
//! engine-canonical active_characters (docs/ordering-inventory.md).

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_positive_float, parse_positive_int, parse_positive_int_range, parse_symbol,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{Animation, ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::floor_div;

mod matrix_step;
#[allow(unused_imports)]
use matrix_step::*;
mod matrix_sim;
#[allow(unused_imports)]
use matrix_sim::*;
mod matrix_build;
#[allow(unused_imports)]
use matrix_build::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct MatrixConfig {
    /// Color for the bottom of the rain column.
    #[arg(long = "highlight-color", default_value = "dbffdb", value_parser = parse_color)]
    pub highlight_color: Color,

    /// Space separated, unquoted, list of colors for the rain gradient. Colors are selected from the gradient randomly. If only one color is provided, the characters will be displayed in that color.
    #[arg(long = "rain-color-gradient", num_args = 1.., value_parser = parse_color,
          default_values = ["92be92", "185318"])]
    pub rain_color_gradient: Vec<Color>,

    /// Space separated, unquoted, list of symbols to use for the rain.
    #[arg(long = "rain-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = [
              "2", "5", "9", "8", "Z", "*", ")", ":", ".", "\"", "=", "+", "-", "¦", "|", "_",
              "ｦ", "ｱ", "ｳ", "ｴ", "ｵ", "ｶ", "ｷ", "ｹ", "ｺ", "ｻ", "ｼ", "ｽ", "ｾ", "ｿ", "ﾀ", "ﾂ",
              "ﾃ", "ﾅ", "ﾆ", "ﾇ", "ﾈ", "ﾊ", "ﾋ", "ﾎ", "ﾏ", "ﾐ", "ﾑ", "ﾒ", "ﾓ", "ﾔ", "ﾕ", "ﾗ",
              "ﾘ", "ﾜ",
          ])]
    pub rain_symbols: Vec<String>,

    /// Range for the speed of the falling rain as determined by the delay between rows. Actual delay is randomly selected from the range.
    #[arg(long = "rain-fall-delay-range", default_value = "1-3", value_parser = parse_positive_int_range)]
    pub rain_fall_delay_range: (i64, i64),

    /// Range of frames to wait between adding new rain columns.
    #[arg(long = "rain-column-delay-range", default_value = "1-2", value_parser = parse_positive_int_range)]
    pub rain_column_delay_range: (i64, i64),

    /// Time, in seconds, to display the rain effect before transitioning to the input text.
    #[arg(long = "rain-time", default_value_t = 2, value_parser = parse_positive_int)]
    pub rain_time: i64,

    /// Chance of swapping a character's symbol on each tick.
    #[arg(long = "symbol-swap-chance", default_value_t = 0.02, value_parser = parse_positive_float)]
    pub symbol_swap_chance: f64,

    /// Chance of swapping a character's color on each tick.
    #[arg(long = "color-swap-chance", default_value_t = 0.008, value_parser = parse_positive_float)]
    pub color_swap_chance: f64,

    /// Number of frames to wait between resolving the next group of characters. This is used to adjust the speed of the final resolve phase.
    #[arg(long = "resolve-delay", default_value_t = 1, value_parser = parse_positive_int)]
    pub resolve_delay: i64,

    /// Space separated, unquoted, list of colors for the character gradient (applied from bottom to top). If only one color is provided, the characters will be displayed in that color.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["92be92", "336b33"])]
    pub final_gradient_stops: Vec<Color>,

    /// Space separated, unquoted, list of the number of gradient steps to use. More steps will create a smoother and longer gradient animation.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Number of frames to display each gradient step. Increase to slow down the gradient animation.
    #[arg(long = "final-gradient-frames", default_value_t = 1, value_parser = parse_positive_int)]
    pub final_gradient_frames: i64,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "radial", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ColumnPhase {
    Rain,
    Fill,
}
/// MatrixIterator.RainColumn.
pub(super) struct RainColumn {
    pub(super) characters: Vec<CharId>,
    pub(super) pending_characters: Vec<CharId>,
    pub(super) visible_characters: Vec<CharId>,
    pub(super) phase: ColumnPhase,
    pub(super) column_drop_chance: f64,
    pub(super) base_rain_fall_delay: i64,
    pub(super) active_rain_fall_delay: i64,
    pub(super) length: usize,
    pub(super) hold_time: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Rain,
    Fill,
    Resolve,
}
pub struct Matrix {
    pub(super) config: MatrixConfig,
    /// Column arena; pending/active/full hold indices (identity semantics).
    pub(super) columns: Vec<RainColumn>,
    pub(super) pending_columns: Vec<usize>,
    pub(super) active_columns: Vec<usize>,
    pub(super) full_columns: Vec<usize>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    /// Gradient(*rain_color_gradient, steps=6).spectrum.
    pub(super) rain_colors: Vec<Color>,
    pub(super) column_delay: i64,
    pub(super) resolve_delay: i64,
    pub(super) final_frame_shown: bool,
    pub(super) rain_complete: bool,
    pub(super) phase: Phase,
    /// time.time() taken after build (effect_matrix.py:430).
    pub(super) rain_start: f64,
}
/// Animation.set_appearance shorthand (upstream character.animation.set_appearance).
pub(super) fn set_appearance(ctx: &mut EngineCtx, id: CharId, symbol: &str, colors: ColorPair) {
    let ch = &mut ctx.terminal.arena[id.0 as usize];
    let uses_pre = ch.uses_input_preexisting_colors;
    ch.animation
        .set_appearance(&ch.input_symbol, uses_pre, Some(symbol), Some(colors));
}
