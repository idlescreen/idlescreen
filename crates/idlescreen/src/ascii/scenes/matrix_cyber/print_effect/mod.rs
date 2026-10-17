//! print, ported from effects/effect_print.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
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

mod print_effect_sim;
mod print_effect_step;

#[derive(Debug, Clone)]
pub struct PrintConfig {
    pub print_head_return_speed: f64,
    pub print_speed: i64,
    pub print_head_easing: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for PrintConfig {
    fn default() -> Self {
        Self {
            print_head_return_speed: 4.0,
            print_speed: 6,
            print_head_easing: Easing::InOutQuad,
            final_gradient_stops: vec![
                hex_color("02b8bd"),
                hex_color("c1f0e3"),
                hex_color("00ffa0"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Diagonal,
        }
    }
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
