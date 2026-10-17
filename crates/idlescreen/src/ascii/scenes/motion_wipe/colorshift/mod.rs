//! colorshift, ported from effects/effect_colorshift.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::geometry;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod colorshift_step;

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ColorShiftConfig {
    pub gradient_stops: Vec<Color>,
    pub gradient_steps: Vec<i64>,
    pub gradient_frames: i64,
    pub no_travel: bool,
    pub travel_direction: GradientDirection,
    pub reverse_travel_direction: bool,
    pub no_loop: bool,
    pub cycles: i64,
    pub skip_final_gradient: bool,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for ColorShiftConfig {
    fn default() -> Self {
        Self {
            gradient_stops: vec![
                hex_color("e81416"),
                hex_color("ffa500"),
                hex_color("faeb36"),
                hex_color("79c314"),
                hex_color("487de7"),
                hex_color("4b369d"),
                hex_color("70369d"),
            ],
            gradient_steps: vec![12],
            gradient_frames: 1,
            no_travel: false,
            travel_direction: GradientDirection::Radial,
            reverse_travel_direction: false,
            no_loop: false,
            cycles: 2,
            skip_final_gradient: false,
            final_gradient_stops: vec![
                hex_color("e81416"),
                hex_color("ffa500"),
                hex_color("faeb36"),
                hex_color("79c314"),
                hex_color("487de7"),
                hex_color("4b369d"),
                hex_color("70369d"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
pub struct ColorShift {
    pub(super) config: ColorShiftConfig,
    pub(super) character_final_color_map: HashMap<CharId, Color>,
    pub(super) loop_tracker_map: HashMap<CharId, i64>,
}
impl ColorShift {
    pub fn new(config: ColorShiftConfig) -> Self {
        ColorShift {
            config,
            character_final_color_map: HashMap::new(),
            loop_tracker_map: HashMap::new(),
        }
    }
}
impl EffectHooks for ColorShift {
    /// ColorShiftIterator.loop_tracker.
    fn dispatch_callback(
        &mut self,
        ctx: &mut EngineCtx,
        character: CharId,
        _callback: &EffectCallback,
    ) {
        let count = {
            let entry = self.loop_tracker_map.entry(character).or_insert(0);
            *entry += 1;
            *entry
        };
        if self.config.cycles == 0 || count < self.config.cycles {
            ctx.activate_scene(self, character, "gradient");
        } else if !self.config.skip_final_gradient {
            ctx.activate_scene(self, character, "final_gradient");
        }
    }
}
