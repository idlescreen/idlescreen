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

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
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

mod matrix_build;
mod matrix_sim;
mod matrix_step;

#[derive(Debug, Clone)]
pub struct MatrixConfig {
    pub highlight_color: Color,
    pub rain_color_gradient: Vec<Color>,
    pub rain_symbols: Vec<String>,
    pub rain_fall_delay_range: (i64, i64),
    pub rain_column_delay_range: (i64, i64),
    pub rain_time: i64,
    pub symbol_swap_chance: f64,
    pub color_swap_chance: f64,
    pub resolve_delay: i64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_frames: i64,
    pub final_gradient_direction: GradientDirection,
}
impl Default for MatrixConfig {
    fn default() -> Self {
        Self {
            highlight_color: hex_color("dbffdb"),
            rain_color_gradient: vec![hex_color("92be92"), hex_color("185318")],
            rain_symbols: [
                "2", "5", "9", "8", "Z", "*", ")", ":", ".", "\"", "=", "+", "-", "¦", "|", "_",
                "ｦ", "ｱ", "ｳ", "ｴ", "ｵ", "ｶ", "ｷ", "ｹ", "ｺ", "ｻ", "ｼ", "ｽ", "ｾ", "ｿ", "ﾀ", "ﾂ",
                "ﾃ", "ﾅ", "ﾆ", "ﾇ", "ﾈ", "ﾊ", "ﾋ", "ﾎ", "ﾏ", "ﾐ", "ﾑ", "ﾒ", "ﾓ", "ﾔ", "ﾕ", "ﾗ",
                "ﾘ", "ﾜ",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            rain_fall_delay_range: (1, 3),
            rain_column_delay_range: (1, 2),
            rain_time: 2,
            symbol_swap_chance: 0.02,
            color_swap_chance: 0.008,
            resolve_delay: 1,
            final_gradient_stops: vec![hex_color("92be92"), hex_color("336b33")],
            final_gradient_steps: vec![12],
            final_gradient_frames: 1,
            final_gradient_direction: GradientDirection::Radial,
        }
    }
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
