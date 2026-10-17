//! bubbles, ported from effects/effect_bubbles.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::{self, Coord};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod bubbles_sim;
mod bubbles_step;

/// pop_condition choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopCondition {
    Row,
    Bottom,
    Anywhere,
}
#[derive(Debug, Clone)]
pub struct BubblesConfig {
    pub rainbow: bool,
    pub bubble_colors: Vec<Color>,
    pub pop_color: Color,
    pub bubble_speed: f64,
    pub bubble_delay: i64,
    pub pop_condition: PopCondition,
    pub movement_easing: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for BubblesConfig {
    fn default() -> Self {
        Self {
            rainbow: false,
            bubble_colors: vec![
                hex_color("d33aff"),
                hex_color("7395c4"),
                hex_color("43c2a7"),
                hex_color("02ff7f"),
            ],
            pop_color: hex_color("ffffff"),
            bubble_speed: 1.1,
            bubble_delay: 4,
            pop_condition: PopCondition::Row,
            movement_easing: Easing::InOutSine,
            final_gradient_stops: vec![hex_color("d33aff"), hex_color("02ff7f")],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Diagonal,
        }
    }
}
/// BubblesIterator.Bubble state (methods live on Bubbles for hooks access).
pub(super) struct Bubble {
    pub(super) characters: Vec<CharId>,
    pub(super) radius: i64,
    pub(super) anchor_char: CharId,
    pub(super) lowest_row: i64,
    pub(super) landed: bool,
}
#[allow(clippy::struct_field_names)]
pub struct Bubbles {
    pub(super) config: BubblesConfig,
    pub(super) bubbles: Vec<Bubble>,
    pub(super) animating_bubbles: Vec<Bubble>,
    pub(super) rainbow_gradient: Gradient,
    pub(super) character_final_color_map: HashMap<CharId, Color>,
    pub(super) steps_since_last_bubble: i64,
}
