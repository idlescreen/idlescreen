//! overflow, ported from effects/effect_overflow.py.

use std::collections::{HashMap, VecDeque};

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps, parse_positive_int,
    parse_positive_int_range,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::ExistingColorHandling;
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup};
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::floor_div;

mod overflow_step;
#[allow(unused_imports)]
use overflow_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct OverflowConfig {
    /// Space separated, unquoted, list of colors for the overflow gradient.
    #[arg(long = "overflow-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["f2ebc0", "8dbfb3", "f2ebc0"])]
    pub overflow_gradient_stops: Vec<Color>,

    /// Number of cycles to overflow the text.
    #[arg(long = "overflow-cycles-range", default_value = "2-4", value_parser = parse_positive_int_range)]
    pub overflow_cycles_range: (i64, i64),

    /// Speed of the overflow effect.
    #[arg(long = "overflow-speed", default_value_t = 3, value_parser = parse_positive_int)]
    pub overflow_speed: i64,

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
/// OverflowIterator.Row.
pub(super) struct Row {
    pub(super) characters: Vec<CharId>,
    pub(super) final_: bool,
}
pub struct Overflow {
    pub(super) config: OverflowConfig,
    pub(super) pending_rows: VecDeque<Row>,
    pub(super) active_rows: Vec<Row>,
    pub(super) character_final_color_map: HashMap<CharId, Color>,
    pub(super) delay: i64,
    pub(super) overflow_gradient: Option<Gradient>,
}
impl Row {
    /// Row.move_up.
    pub(super) fn move_up(&self, ctx: &mut EngineCtx) {
        for &id in &self.characters {
            let motion = &mut ctx.terminal.arena[id.0 as usize].motion;
            let current = motion.current_coord;
            motion.set_coordinate(Coord::new(current.column, current.row + 1));
        }
    }

    /// Row.setup.
    pub(super) fn setup(&self, ctx: &mut EngineCtx) {
        for &id in &self.characters {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let column = ch.input_coord.column;
            ch.motion.set_coordinate(Coord::new(column, 0));
        }
    }

    /// Row.set_color.
    pub(super) fn set_color(
        &self,
        ctx: &mut EngineCtx,
        fg_color: Option<Color>,
        bg_color: Option<Color>,
    ) {
        for &id in &self.characters {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let uses_pre = ch.uses_input_preexisting_colors;
            ch.animation.set_appearance(
                &ch.input_symbol,
                uses_pre,
                None,
                Some(ColorPair::new(fg_color.clone(), bg_color.clone())),
            );
        }
    }
}
impl Overflow {
    pub fn new(config: OverflowConfig) -> Self {
        Overflow {
            config,
            pending_rows: VecDeque::new(),
            active_rows: Vec::new(),
            character_final_color_map: HashMap::new(),
            delay: 0,
            overflow_gradient: None,
        }
    }
}
impl EffectHooks for Overflow {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
