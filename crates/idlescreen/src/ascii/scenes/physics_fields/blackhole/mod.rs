//! blackhole, ported from effects/effect_blackhole.py.

use std::collections::{HashMap, HashSet};

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, SyncMetric, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::{floor_div, round_half_even};

mod blackhole_step;
#[allow(unused_imports)]
use blackhole_step::*;
mod blackhole_sim;
#[allow(unused_imports)]
use blackhole_sim::*;
mod blackhole_build;
#[allow(unused_imports)]
use blackhole_build::*;
mod blackhole_fx;
#[allow(unused_imports)]
use blackhole_fx::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct BlackholeConfig {
    /// Color for the stars that comprise the blackhole border.
    #[arg(long = "blackhole-color", default_value = "ffffff", value_parser = parse_color)]
    pub blackhole_color: Color,

    /// List of colors from which character colors will be chosen and applied after the explosion, but before the cooldown to final color.
    #[arg(long = "star-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["ffcc0d", "ff7326", "ff194d", "bf2669", "702a8c", "049dbf"])]
    pub star_colors: Vec<Color>,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["8A008A", "00D1FF", "ffffff"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["9"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "diagonal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Phase {
    Forming,
    Consuming,
    Collapsing,
    Exploding,
    Complete,
}
pub struct Blackhole {
    pub(super) config: BlackholeConfig,
    pub(super) blackhole_chars: Vec<CharId>,
    pub(super) awaiting_consumption_chars: Vec<CharId>,
    pub(super) blackhole_radius: i64,
    pub(super) character_final_color_map: HashMap<CharId, Color>,
    pub(super) formation_delay: i64,
    pub(super) f_delay: i64,
    pub(super) phase: Phase,
    pub(super) awaiting_blackhole_chars: Vec<CharId>,
}
impl Blackhole {
    pub fn new(config: BlackholeConfig) -> Self {
        Blackhole {
            config,
            blackhole_chars: Vec::new(),
            awaiting_consumption_chars: Vec::new(),
            blackhole_radius: 0,
            character_final_color_map: HashMap::new(),
            formation_delay: 0,
            f_delay: 0,
            phase: Phase::Forming,
            awaiting_blackhole_chars: Vec::new(),
        }
    }
}
