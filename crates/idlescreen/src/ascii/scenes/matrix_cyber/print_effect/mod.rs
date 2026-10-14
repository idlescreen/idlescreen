//! print, ported from effects/effect_print.py.

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
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod print_effect_step;
#[allow(unused_imports)]
use print_effect_step::*;
mod print_effect_sim;
#[allow(unused_imports)]
use print_effect_sim::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct PrintConfig {
    /// Speed of the print head when performing a carriage return.
    #[arg(long = "print-head-return-speed", default_value_t = 4.0, value_parser = parse_positive_float)]
    pub print_head_return_speed: f64,

    /// Speed of the print head when printing characters.
    #[arg(long = "print-speed", default_value_t = 6, value_parser = parse_positive_int)]
    pub print_speed: i64,

    /// Easing function to use for print head movement.
    #[arg(long = "print-head-easing", default_value = "in_out_quad", value_parser = parse_easing)]
    pub print_head_easing: Easing,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["02b8bd", "c1f0e3", "00ffa0"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "diagonal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
/// PrintIterator.Row (plain struct over CharIds; scene/coord setup happens in
/// Print::make_row so it can borrow the engine).
pub(super) struct Row {
    pub(super) untyped_chars: Vec<CharId>,
    pub(super) typed_chars: Vec<CharId>,
}
pub(super) const SET_INVISIBLE_CALLBACK: u32 = 0;
pub struct Print {
    pub(super) config: PrintConfig,
    pub(super) pending_rows: Vec<Row>,
    pub(super) processed_rows: Vec<Row>,
    pub(super) typing_head: CharId,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) current_row: Row,
    pub(super) typing: bool,
    pub(super) last_column: i64,
}
impl Row {
    /// Row.move_up.
    pub(super) fn move_up(&self, ctx: &mut EngineCtx) {
        for &id in &self.typed_chars {
            let motion = &mut ctx.terminal.arena[id.0 as usize].motion;
            let current = motion.current_coord;
            motion.set_coordinate(Coord::new(current.column, current.row + 1));
        }
    }

    /// Row.type_char.
    pub(super) fn type_char(&mut self) -> Option<CharId> {
        if self.untyped_chars.is_empty() {
            return None;
        }
        let next_char = self.untyped_chars.remove(0);
        self.typed_chars.push(next_char);
        Some(next_char)
    }
}
