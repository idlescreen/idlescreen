//! decrypt, ported from effects/effect_decrypt.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps, parse_positive_int,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod decrypt_step;
#[allow(unused_imports)]
use decrypt_step::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct DecryptConfig {
    /// Number of characters typed per keystroke.
    #[arg(long = "typing-speed", default_value_t = 12, value_parser = parse_positive_int)]
    pub typing_speed: i64,

    /// Space separated, unquoted, list of colors for the ciphertext. Color will be randomly selected for each character.
    #[arg(long = "ciphertext-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["008000", "00cb00", "00ff00"])]
    pub ciphertext_colors: Vec<Color>,

    /// Space separated, unquoted, list of colors for the character gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["eda000"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
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
        for n in 33u32..127 {
            self.encrypted_symbols
                .push(char::from_u32(n).must_val().to_string());
        }
        for n in 9608u32..9632 {
            self.encrypted_symbols
                .push(char::from_u32(n).must_val().to_string());
        }
        for n in 9472u32..9599 {
            self.encrypted_symbols
                .push(char::from_u32(n).must_val().to_string());
        }
        for n in 174u32..452 {
            self.encrypted_symbols
                .push(char::from_u32(n).must_val().to_string());
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
                ch.animation.input_fg_color.clone(),
                ch.animation.input_bg_color.clone(),
                ch.uses_input_preexisting_colors,
            )
        };
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation
                .new_scene(false, None, None, "fast_decrypt", uses_pre);
        }
        let color = ctx.rng.choice(&self.config.ciphertext_colors).clone();
        for _ in 0..80 {
            let symbol = ctx.rng.choice(&self.encrypted_symbols).clone();
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation
                .scenes
                .get_mut("fast_decrypt")
                .must_val()
                .add_frame(
                    &symbol,
                    2,
                    VisualParams {
                        colors: Some(ColorPair::new(Some(color.clone()), None)),
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
                .must_val()
                .add_frame(
                    &symbol,
                    duration,
                    VisualParams {
                        colors: Some(ColorPair::new(Some(color.clone()), None)),
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
        let white = Color::from_hex("ffffff").must_val();
        if dynamic {
            let fg_gradient = match &input_fg {
                Some(fg) => Some(
                    Gradient::with_steps(&[white.clone(), fg.clone()], 10, false)
                        .map_err(EngineError::Other)?,
                ),
                None => None,
            };
            let bg_gradient = match &input_bg {
                Some(bg) => Some(
                    Gradient::with_steps(&[white.clone(), bg.clone()], 10, false)
                        .map_err(EngineError::Other)?,
                ),
                None => None,
            };
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            let scene = ch.animation.scenes.get_mut("discovered").must_val();
            if fg_gradient.is_some() || bg_gradient.is_some() {
                scene
                    .apply_gradient_to_symbols(
                        &[input_symbol.clone()],
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
                .must_val()
                .fg_color
                .clone()
                .must_val();
            let discovered_gradient =
                Gradient::with_steps(&[white, final_fg], 10, false).map_err(EngineError::Other)?;
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.animation
                .scenes
                .get_mut("discovered")
                .must_val()
                .apply_gradient_to_symbols(
                    &[input_symbol.clone()],
                    5,
                    Some(&discovered_gradient),
                    None,
                )
                .map_err(EngineError::Other)?;
        }
        Ok(())
    }
}
