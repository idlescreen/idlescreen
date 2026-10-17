//! slide, ported from effects/effect_slide.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod slide_step;

/// Slide grouping: `"row"`, `"column"`, or `"diagonal"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlideGrouping {
    Row,
    Column,
    Diagonal,
}
#[derive(Debug, Clone)]
pub struct SlideConfig {
    pub movement_speed: f64,
    pub grouping: SlideGrouping,
    pub gap: i64,
    pub reverse_direction: bool,
    pub merge: bool,
    pub movement_easing: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_frames: i64,
    pub final_gradient_direction: GradientDirection,
}
impl Default for SlideConfig {
    fn default() -> Self {
        Self {
            movement_speed: 0.8,
            grouping: SlideGrouping::Row,
            gap: 2,
            reverse_direction: false,
            merge: false,
            movement_easing: Easing::InOutQuad,
            final_gradient_stops: vec![
                hex_color("833ab4"),
                hex_color("fd1d1d"),
                hex_color("fcb045"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_frames: 6,
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
pub struct Slide {
    pub(super) config: SlideConfig,
    pub(super) pending_groups: Vec<Vec<CharId>>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) active_groups: Vec<Vec<CharId>>,
    pub(super) current_gap: i64,
}
impl Slide {
    pub fn new(config: SlideConfig) -> Self {
        Slide {
            config,
            pending_groups: Vec::new(),
            character_final_color_map: HashMap::new(),
            active_groups: Vec::new(),
            current_gap: 0,
        }
    }
}
impl EffectHooks for Slide {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Slide {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.build_scene(ctx)
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.pending_groups.is_empty()
            || !ctx.active_characters.is_empty()
            || !self.active_groups.is_empty()
        {
            if self.current_gap == self.config.gap && !self.pending_groups.is_empty() {
                self.active_groups.push(self.pending_groups.remove(0));
                self.current_gap = 0;
            } else if !self.pending_groups.is_empty() {
                self.current_gap += 1;
            }
            for group_index in 0..self.active_groups.len() {
                if !self.active_groups[group_index].is_empty() {
                    let next_char = self.active_groups[group_index].remove(0);
                    ctx.terminal.set_character_visibility(next_char, true);
                    ctx.activate_path(self, next_char, "input_path");
                    ctx.active_characters.insert(next_char);
                }
            }
            self.active_groups.retain(|group| !group.is_empty());
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Slide {
    fn default() -> Self {
        Self::new(SlideConfig::default())
    }
}
