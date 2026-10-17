//! blackhole, ported from effects/effect_blackhole.py.

use std::collections::{HashMap, HashSet};

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
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

mod blackhole_build;
mod blackhole_fx;
mod blackhole_sim;
mod blackhole_step;

#[derive(Debug, Clone)]
pub struct BlackholeConfig {
    pub blackhole_color: Color,
    pub star_colors: Vec<Color>,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for BlackholeConfig {
    fn default() -> Self {
        Self {
            blackhole_color: hex_color("ffffff"),
            star_colors: vec![
                hex_color("ffcc0d"),
                hex_color("ff7326"),
                hex_color("ff194d"),
                hex_color("bf2669"),
                hex_color("702a8c"),
                hex_color("049dbf"),
            ],
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("ffffff"),
            ],
            final_gradient_steps: vec![9],
            final_gradient_direction: GradientDirection::Diagonal,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Phase {
    Forming,
    Consuming,
    Collapsing,
    Exploding,
    Complete,
}
#[allow(clippy::struct_field_names)]
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
