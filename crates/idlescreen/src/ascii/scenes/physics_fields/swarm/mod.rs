//! swarm, ported from effects/effect_swarm.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_ratio, parse_positive_int_range,
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
use ttfx::utils::geometry::{self, Coord};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::{floor_div, round_half_even};

mod swarm_step;
#[allow(unused_imports)]
use swarm_step::*;
mod swarm_sim;
#[allow(unused_imports)]
use swarm_sim::*;
mod swarm_build;
#[allow(unused_imports)]
use swarm_build::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct SwarmConfig {
    /// Space separated, unquoted, list of colors for the swarms.
    #[arg(long = "base-color", num_args = 1.., value_parser = parse_color,
          default_values = ["31a0d4"])]
    pub base_color: Vec<Color>,

    /// Color for the character flash. Characters flash when moving.
    #[arg(long = "flash-color", default_value = "f2ea79", value_parser = parse_color)]
    pub flash_color: Color,

    /// Percent of total characters in each swarm.
    #[arg(long = "swarm-size", default_value_t = 0.30, value_parser = parse_non_negative_ratio)]
    pub swarm_size: f64,

    /// Percent of characters in a swarm that move as a group.
    #[arg(long = "swarm-coordination", default_value_t = 0.80, value_parser = parse_non_negative_ratio)]
    pub swarm_coordination: f64,

    /// Range of the number of areas where characters will swarm.
    #[arg(long = "swarm-area-count-range", default_value = "2-4", value_parser = parse_positive_int_range)]
    pub swarm_area_count_range: (i64, i64),

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["31b900", "f0ff65"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "horizontal", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
pub struct Swarm {
    pub(super) config: SwarmConfig,
    pub(super) swarms: Vec<Vec<CharId>>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) call_next: bool,
    pub(super) active_swarm_area: String,
    pub(super) current_swarm: Vec<CharId>,
}
impl Swarm {
    pub fn new(config: SwarmConfig) -> Self {
        Swarm {
            config,
            swarms: Vec::new(),
            character_final_color_map: HashMap::new(),
            call_next: true,
            active_swarm_area: "0_swarm_area".to_string(),
            current_swarm: Vec::new(),
        }
    }

    /// SwarmIterator.make_swarms.
    pub(super) fn make_swarms(&mut self, ctx: &mut EngineCtx, swarm_size: i64) {
        let mut unswarmed_characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::BottomToTopRightToLeft)
        };
        while !unswarmed_characters.is_empty() {
            let mut new_swarm: Vec<CharId> = Vec::new();
            for _ in 0..swarm_size {
                if let Some(id) = unswarmed_characters.pop() {
                    new_swarm.push(id);
                } else {
                    break;
                }
            }
            self.swarms.push(new_swarm);
        }
        let final_swarm = self.swarms.pop().must_val();
        if (final_swarm.len() as i64) < floor_div(swarm_size, 2) {
            self.swarms.last_mut().must_val().extend(final_swarm);
        } else {
            self.swarms.push(final_swarm);
        }
    }
}
/// int(s[0]) on a path id string (effect_swarm.py's first-character parse).
pub(super) fn first_char_digit(s: &str) -> i64 {
    s.chars().next().and_then(|c| c.to_digit(10)).must_val() as i64
}
impl EffectHooks for Swarm {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
