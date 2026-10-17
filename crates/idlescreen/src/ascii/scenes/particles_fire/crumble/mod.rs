//! crumble, ported from effects/effect_crumble.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
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

mod crumble_sim;
mod crumble_step;

#[derive(Debug, Clone)]
pub struct CrumbleConfig {
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for CrumbleConfig {
    fn default() -> Self {
        Self {
            final_gradient_stops: vec![hex_color("5CE1FF"), hex_color("FF8C00")],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Diagonal,
        }
    }
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

impl Default for Crumble {
    fn default() -> Self {
        Self::new(CrumbleConfig::default())
    }
}
