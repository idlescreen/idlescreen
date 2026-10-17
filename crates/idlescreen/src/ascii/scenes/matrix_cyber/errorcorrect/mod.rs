//! errorcorrect, ported from effects/effect_errorcorrect.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, SyncMetric, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod errorcorrect_sim;
mod errorcorrect_step;

#[derive(Debug, Clone)]
pub struct ErrorCorrectConfig {
    pub error_pairs: f64,
    pub swap_delay: i64,
    pub error_color: Color,
    pub correct_color: Color,
    pub movement_speed: f64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for ErrorCorrectConfig {
    fn default() -> Self {
        Self {
            error_pairs: 0.1,
            swap_delay: 3,
            error_color: hex_color("e74c3c"),
            correct_color: hex_color("45bf55"),
            movement_speed: 1.5,
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("FFFFFF"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
pub(super) const BLOCK_WIPE_START: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
pub(super) const BLOCK_WIPE_END: [&str; 7] = ["▇", "▆", "▅", "▄", "▃", "▂", "▁"];
pub struct ErrorCorrect {
    pub(super) config: ErrorCorrectConfig,
    pub(super) swapped: Vec<(CharId, CharId)>,
    pub(super) swap_delay: i64,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
}
impl ErrorCorrect {
    pub fn new(config: ErrorCorrectConfig) -> Self {
        ErrorCorrect {
            config,
            swapped: Vec::new(),
            swap_delay: 0,
            character_final_color_map: HashMap::new(),
        }
    }

    /// ErrorCorrectIterator._get_dynamic_final_scene.
    pub(super) fn get_dynamic_final_scene(
        &self,
        ctx: &mut EngineCtx,
        id: CharId,
    ) -> Result<String, EngineError> {
        let (input_symbol, input_fg, input_bg, uses_pre) = {
            let ch = &ctx.terminal.arena[id.0 as usize];
            (
                ch.input_symbol.clone(),
                ch.animation.input_fg_color,
                ch.animation.input_bg_color,
                ch.uses_input_preexisting_colors,
            )
        };
        let final_scene = {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation.new_scene(false, None, None, "", uses_pre)
        };
        let fg_gradient = match &input_fg {
            Some(fg) => Some(
                Gradient::with_steps(&[self.config.correct_color, *fg], 10, false)
                    .map_err(EngineError::Other)?,
            ),
            None => None,
        };
        let bg_gradient = match &input_bg {
            Some(bg) => Some(
                Gradient::with_steps(&[self.config.correct_color, *bg], 10, false)
                    .map_err(EngineError::Other)?,
            ),
            None => None,
        };
        let ch = &mut ctx.terminal.arena[id.0 as usize];
        let scene = ch
            .animation
            .scenes
            .get_mut(&final_scene)
            .ok_or_else(|| EngineError::Other(String::new()))?;
        if fg_gradient.is_some() || bg_gradient.is_some() {
            scene
                .apply_gradient_to_symbols(
                    std::slice::from_ref(&input_symbol),
                    3,
                    fg_gradient.as_ref(),
                    bg_gradient.as_ref(),
                )
                .map_err(EngineError::Other)?;
        } else {
            scene
                .add_frame(
                    &input_symbol,
                    3,
                    VisualParams {
                        colors: Some(ColorPair::default()),
                        ..Default::default()
                    },
                )
                .map_err(EngineError::Other)?;
        }
        Ok(final_scene)
    }
}
