//! sweep, ported from effects/effect_sweep.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::{Easing, SequenceEaser};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod sweep_step;

#[derive(Debug, Clone)]
pub struct SweepConfig {
    pub sweep_symbols: Vec<String>,
    pub first_sweep_direction: CharacterGroup,
    pub second_sweep_direction: CharacterGroup,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for SweepConfig {
    fn default() -> Self {
        Self {
            sweep_symbols: vec![
                "█".to_string(),
                "▓".to_string(),
                "▒".to_string(),
                "░".to_string(),
            ],
            first_sweep_direction: CharacterGroup::ColumnRightToLeft,
            second_sweep_direction: CharacterGroup::ColumnLeftToRight,
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("ffffff"),
            ],
            final_gradient_steps: vec![8],
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
#[allow(clippy::struct_field_names)]
pub struct Sweep {
    pub(super) config: SweepConfig,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) dynamic_second_sweep_palette: Vec<Color>,
    pub(super) complete: bool,
    pub(super) first_phase: bool,
    pub(super) easer: Option<SequenceEaser<Vec<CharId>>>,
    pub(super) groups_second_sweep: Vec<Vec<CharId>>,
}
impl Sweep {
    pub fn new(config: SweepConfig) -> Self {
        Sweep {
            config,
            character_final_color_map: HashMap::new(),
            dynamic_second_sweep_palette: Vec::new(),
            complete: false,
            first_phase: true,
            easer: None,
            groups_second_sweep: Vec::new(),
        }
    }
}
impl EffectHooks for Sweep {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
