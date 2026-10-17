//! decrypt, ported from effects/effect_decrypt.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod decrypt_step;

#[derive(Debug, Clone)]
pub struct DecryptConfig {
    pub typing_speed: i64,
    pub ciphertext_colors: Vec<Color>,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for DecryptConfig {
    fn default() -> Self {
        Self {
            typing_speed: 12,
            ciphertext_colors: vec![
                hex_color("008000"),
                hex_color("00cb00"),
                hex_color("00ff00"),
            ],
            final_gradient_stops: vec![hex_color("eda000")],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Typing,
    Decrypting,
}
pub struct Decrypt {
    pub(super) config: DecryptConfig,
    pub(super) typing_pending_chars: Vec<CharId>,
    /// Upstream is a set; membership only feeds `active_characters`, which is
    /// canonically ordered, so insertion-ordered Vec is equivalent.
    pub(super) decrypting_pending_chars: Vec<CharId>,
    pub(super) phase: Phase,
    pub(super) encrypted_symbols: Vec<String>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
}
impl Decrypt {
    pub fn new(config: DecryptConfig) -> Self {
        let mut effect = Decrypt {
            config,
            typing_pending_chars: Vec::new(),
            decrypting_pending_chars: Vec::new(),
            phase: Phase::Typing,
            encrypted_symbols: Vec::new(),
            character_final_color_map: HashMap::new(),
        };
        effect.make_encrypted_symbols();
        effect
    }

    /// DecryptIterator.make_encrypted_symbols (_DecryptChars ranges).
    pub(super) fn make_encrypted_symbols(&mut self) {
        for range in [33u32..127, 9608..9632, 9472..9599, 174..452] {
            for n in range {
                if let Some(ch) = char::from_u32(n) {
                    self.encrypted_symbols.push(ch.to_string());
                }
            }
        }
    }

    /// DecryptIterator.make_decrypting_animation_scenes.
    pub(super) fn make_decrypting_animation_scenes(
        &mut self,
        ctx: &mut EngineCtx,
        id: CharId,
    ) -> Result<(), EngineError> {
        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let (input_symbol, input_fg, input_bg, uses_pre) = {
            let ch = &ctx.terminal.arena[id.0 as usize];
            (
                ch.input_symbol.clone(),
                ch.animation.input_fg_color,
                ch.animation.input_bg_color,
                ch.uses_input_preexisting_colors,
            )
        };
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation
                .new_scene(false, None, None, "fast_decrypt", uses_pre);
        }
        let color = *ctx.rng.choice(&self.config.ciphertext_colors);
        for _ in 0..80 {
            let symbol = ctx.rng.choice(&self.encrypted_symbols).clone();
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation
                .scenes
                .get_mut("fast_decrypt")
                .ok_or_else(|| EngineError::Other(String::new()))?
                .add_frame(
                    &symbol,
                    2,
                    VisualParams {
                        colors: Some(ColorPair::new(Some(color), None)),
                        ..Default::default()
                    },
                )
                .map_err(EngineError::Other)?;
        }
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation
                .new_scene(false, None, None, "slow_decrypt", uses_pre);
        }
        for _ in 0..ctx.rng.randint(1, 15) {
            // 1-15 longer duration units
            let symbol = ctx.rng.choice(&self.encrypted_symbols).clone();
            // 30% chance of extra long duration
            // wide duration range reduces 'waves' in the animation
            // shorter duration creates flipping effect
            let duration = if ctx.rng.randint(0, 100) <= 30 {
                ctx.rng.randrange(10, 20)
            } else {
                ctx.rng.randrange(3, 6)
            };
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation
                .scenes
                .get_mut("slow_decrypt")
                .ok_or_else(|| EngineError::Other(String::new()))?
                .add_frame(
                    &symbol,
                    duration,
                    VisualParams {
                        colors: Some(ColorPair::new(Some(color), None)),
                        ..Default::default()
                    },
                )
                .map_err(EngineError::Other)?;
        }
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation
                .new_scene(false, None, None, "discovered", uses_pre);
        }
        let white = hex_color("ffffff");
        if dynamic {
            let fg_gradient = match &input_fg {
                Some(fg) => Some(
                    Gradient::with_steps(&[white, *fg], 10, false).map_err(EngineError::Other)?,
                ),
                None => None,
            };
            let bg_gradient = match &input_bg {
                Some(bg) => Some(
                    Gradient::with_steps(&[white, *bg], 10, false).map_err(EngineError::Other)?,
                ),
                None => None,
            };
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let scene = ch
                .animation
                .scenes
                .get_mut("discovered")
                .ok_or_else(|| EngineError::Other(String::new()))?;
            if fg_gradient.is_some() || bg_gradient.is_some() {
                scene
                    .apply_gradient_to_symbols(
                        std::slice::from_ref(&input_symbol),
                        5,
                        fg_gradient.as_ref(),
                        bg_gradient.as_ref(),
                    )
                    .map_err(EngineError::Other)?;
            } else {
                scene
                    .add_frame(
                        &input_symbol,
                        5,
                        VisualParams {
                            colors: Some(ColorPair::default()),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
            }
        } else {
            let final_fg = self
                .character_final_color_map
                .get(&id)
                .ok_or_else(|| EngineError::Other(String::new()))?
                .fg_color
                .ok_or_else(|| EngineError::Other(String::new()))?;
            let discovered_gradient =
                Gradient::with_steps(&[white, final_fg], 10, false).map_err(EngineError::Other)?;
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation
                .scenes
                .get_mut("discovered")
                .ok_or_else(|| EngineError::Other(String::new()))?
                .apply_gradient_to_symbols(
                    std::slice::from_ref(&input_symbol),
                    5,
                    Some(&discovered_gradient),
                    None,
                )
                .map_err(EngineError::Other)?;
        }
        Ok(())
    }
}
