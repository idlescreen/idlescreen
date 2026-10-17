//! waves, ported from effects/effect_waves.py.

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
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod waves_step;

#[derive(Debug, Clone)]
pub struct WavesConfig {
    pub wave_symbols: Vec<String>,
    pub wave_gradient_stops: Vec<Color>,
    pub wave_gradient_steps: Vec<i64>,
    pub wave_count: i64,
    pub wave_length: i64,
    pub wave_direction: CharacterGroup,
    pub wave_easing: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for WavesConfig {
    fn default() -> Self {
        Self {
            wave_symbols: vec![
                "▁".to_string(),
                "▂".to_string(),
                "▃".to_string(),
                "▄".to_string(),
                "▅".to_string(),
                "▆".to_string(),
                "▇".to_string(),
                "█".to_string(),
                "▇".to_string(),
                "▆".to_string(),
                "▅".to_string(),
                "▄".to_string(),
                "▃".to_string(),
                "▂".to_string(),
                "▁".to_string(),
            ],
            wave_gradient_stops: vec![
                hex_color("f0ff65"),
                hex_color("ffb102"),
                hex_color("31a0d4"),
                hex_color("ffb102"),
                hex_color("f0ff65"),
            ],
            wave_gradient_steps: vec![6],
            wave_count: 4,
            wave_length: 1,
            wave_direction: CharacterGroup::ColumnLeftToRight,
            wave_easing: Easing::InOutQuad,
            final_gradient_stops: vec![
                hex_color("ffb102"),
                hex_color("31a0d4"),
                hex_color("f0ff65"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Diagonal,
        }
    }
}
pub struct Waves {
    pub(super) config: WavesConfig,
    pub(super) pending_columns: Vec<Vec<CharId>>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
}
impl Waves {
    pub fn new(config: WavesConfig) -> Self {
        Waves {
            config,
            pending_columns: Vec::new(),
            character_final_color_map: HashMap::new(),
        }
    }
}
impl EffectHooks for Waves {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Waves {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.build_scene(ctx)
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.pending_columns.is_empty() || !ctx.active_characters.is_empty() {
            if !self.pending_columns.is_empty() {
                let next_column = self.pending_columns.remove(0);
                for id in next_column {
                    ctx.terminal.set_character_visibility(id, true);
                    ctx.active_characters.insert(id);
                }
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Waves {
    fn default() -> Self {
        Self::new(WavesConfig::default())
    }
}
