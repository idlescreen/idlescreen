//! crumble, ported from effects/effect_crumble.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{Animation, ExistingColorHandling, SyncMetric, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod crumble_step;
#[allow(unused_imports)]
use crumble_step::*;
mod crumble_sim;
#[allow(unused_imports)]
use crumble_sim::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct CrumbleConfig {
    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["5CE1FF", "FF8C00"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "diagonal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Stage {
    Falling,
    Vacuuming,
    Resetting,
    Complete,
}
pub struct Crumble {
    pub(super) config: CrumbleConfig,
    pub(super) pending_chars: Vec<CharId>,
    pub(super) character_final_color_map: HashMap<CharId, Color>,
    pub(super) fall_delay: i64,
    pub(super) max_fall_delay: i64,
    pub(super) min_fall_delay: i64,
    pub(super) reset: bool,
    pub(super) fall_group_maxsize: i64,
    pub(super) stage: Stage,
    pub(super) unvacuumed_chars: Vec<CharId>,
}
impl Crumble {
    pub fn new(config: CrumbleConfig) -> Self {
        Crumble {
            config,
            pending_chars: Vec::new(),
            character_final_color_map: HashMap::new(),
            fall_delay: 0,
            max_fall_delay: 0,
            min_fall_delay: 0,
            reset: false,
            fall_group_maxsize: 1,
            stage: Stage::Falling,
            unvacuumed_chars: Vec::new(),
        }
    }
}
impl EffectHooks for Crumble {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
