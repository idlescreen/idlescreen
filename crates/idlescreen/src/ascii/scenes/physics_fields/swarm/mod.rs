//! swarm, ported from effects/effect_swarm.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
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

mod swarm_build;
mod swarm_sim;
mod swarm_step;

#[derive(Debug, Clone)]
pub struct SwarmConfig {
    pub base_color: Vec<Color>,
    pub flash_color: Color,
    pub swarm_size: f64,
    pub swarm_coordination: f64,
    pub swarm_area_count_range: (i64, i64),
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            base_color: vec![hex_color("31a0d4")],
            flash_color: hex_color("f2ea79"),
            swarm_size: 0.30,
            swarm_coordination: 0.80,
            swarm_area_count_range: (2, 4),
            final_gradient_stops: vec![hex_color("31b900"), hex_color("f0ff65")],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Horizontal,
        }
    }
}
#[allow(clippy::struct_field_names)]
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
        let Some(final_swarm) = self.swarms.pop() else {
            return;
        };
        if (final_swarm.len() as i64) < floor_div(swarm_size, 2) {
            if let Some(last) = self.swarms.last_mut() {
                last.extend(final_swarm);
            } else {
                self.swarms.push(final_swarm);
            }
        } else {
            self.swarms.push(final_swarm);
        }
    }
}
/// int(s[0]) on a path id string (effect_swarm.py's first-character parse).
pub(super) fn first_char_digit(s: &str) -> i64 {
    s.chars().next().and_then(|c| c.to_digit(10)).unwrap_or(0) as i64
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
