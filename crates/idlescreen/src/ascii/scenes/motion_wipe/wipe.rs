//! wipe, ported from effects/effect_wipe.py.

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

#[derive(Debug, Clone)]
pub struct WipeConfig {
    pub wipe_direction: CharacterGroup,
    pub wipe_delay: i64,
    pub wipe_ease: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_frames: i64,
    pub final_gradient_direction: GradientDirection,
}

#[allow(clippy::struct_field_names)]
pub struct Wipe {
    config: WipeConfig,
    character_final_color_map: HashMap<CharId, ColorPair>,
    easer: Option<SequenceEaser<Vec<CharId>>>,
    wipe_delay: i64,
}

impl Wipe {
    pub fn new(config: WipeConfig) -> Self {
        let wipe_delay = config.wipe_delay;
        Wipe {
            config,
            character_final_color_map: HashMap::new(),
            easer: None,
            wipe_delay,
        }
    }
}

impl EffectHooks for Wipe {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Wipe {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let groups = ctx
            .terminal
            .get_characters_grouped(CharacterFilter::default(), self.config.wipe_direction);
        self.easer = Some(SequenceEaser::new(groups, self.config.wipe_ease, 100));

        let final_gradient = Gradient::new(
            &self.config.final_gradient_stops,
            &self.config.final_gradient_steps,
            false,
            false,
        )
        .map_err(EngineError::Other)?;
        let final_gradient_mapping = final_gradient
            .build_coordinate_color_mapping(
                ctx.terminal.canvas.text_bottom,
                ctx.terminal.canvas.text_top,
                ctx.terminal.canvas.text_left,
                ctx.terminal.canvas.text_right,
                self.config.final_gradient_direction,
            )
            .map_err(EngineError::Other)?;

        let dynamic = ctx.terminal.config.existing_color_handling == ExistingColorHandling::Dynamic;
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        for id in characters {
            let (input_fg, input_bg, input_coord, input_symbol, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.animation.input_fg_color,
                    ch.animation.input_bg_color,
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            let final_colors = if dynamic {
                ColorPair::new(input_fg, input_bg)
            } else {
                ColorPair::new(
                    Some(
                        *final_gradient_mapping
                            .get(&input_coord)
                            .ok_or_else(|| EngineError::Other(String::new()))?,
                    ),
                    None,
                )
            };
            self.character_final_color_map.insert(id, final_colors);

            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.new_scene(false, None, None, "wipe", uses_pre);
                let scene = ch
                    .animation
                    .scenes
                    .get_mut("wipe")
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                if dynamic {
                    let frame_count: i64 = self.config.final_gradient_steps.iter().sum::<i64>() + 1;
                    for _ in 0..frame_count {
                        scene
                            .add_frame(
                                &input_symbol,
                                self.config.final_gradient_frames,
                                VisualParams {
                                    colors: Some(final_colors),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                } else {
                    let final_fg = final_colors
                        .fg_color
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    let wipe_gradient = Gradient::new(
                        &[final_gradient.spectrum[0], final_fg],
                        &self.config.final_gradient_steps,
                        false,
                        false,
                    )
                    .map_err(EngineError::Other)?;
                    scene
                        .apply_gradient_to_symbols(
                            std::slice::from_ref(&input_symbol),
                            self.config.final_gradient_frames,
                            Some(&wipe_gradient),
                            None,
                        )
                        .map_err(EngineError::Other)?;
                }
            }
        }
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        let easer_complete = self.easer.as_ref()?.is_complete();
        if !ctx.active_characters.is_empty() || !easer_complete {
            if self.wipe_delay == 0 {
                let mut easer = self.easer.take()?;
                let step = easer.step();
                for group in step.added {
                    for &id in group {
                        ctx.activate_scene(self, id, "wipe");
                        ctx.terminal.set_character_visibility(id, true);
                        ctx.active_characters.insert(id);
                    }
                }
                for group in step.removed {
                    for &id in group {
                        ctx.deactivate_scene(id, None);
                        ctx.terminal.arena[id.0 as usize]
                            .animation
                            .scenes
                            .get_mut("wipe")?
                            .reset_scene();
                        ctx.terminal.set_character_visibility(id, false);
                    }
                }
                self.easer = Some(easer);
                self.wipe_delay = self.config.wipe_delay;
            } else {
                self.wipe_delay -= 1;
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for WipeConfig {
    fn default() -> Self {
        Self {
            wipe_direction: CharacterGroup::DiagonalTopLeftToBottomRight,
            wipe_delay: 0,
            wipe_ease: Easing::OutExpo,
            final_gradient_stops: vec![
                hex_color("833ab4"),
                hex_color("fd1d1d"),
                hex_color("fcb045"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_frames: 1,
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}

impl Default for Wipe {
    fn default() -> Self {
        Self::new(WipeConfig::default())
    }
}
