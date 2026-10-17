//! highlight, ported from effects/effect_highlight.py.

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{Animation, ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::{Easing, SequenceEaser};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

#[derive(Debug, Clone)]
pub struct HighlightConfig {
    pub highlight_brightness: f64,
    pub highlight_direction: CharacterGroup,
    pub highlight_width: i64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}

pub struct Highlight {
    config: HighlightConfig,
    easer: Option<SequenceEaser<Vec<CharId>>>,
}

impl Highlight {
    pub fn new(config: HighlightConfig) -> Self {
        Highlight {
            config,
            easer: None,
        }
    }
}

impl EffectHooks for Highlight {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Highlight {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let groups = ctx
            .terminal
            .get_characters_grouped(CharacterFilter::default(), self.config.highlight_direction);
        self.easer = Some(SequenceEaser::new(groups, Easing::InOutCirc, 100));

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
            let (base_color, input_bg_color) = if dynamic {
                (input_fg, input_bg)
            } else {
                (
                    Some(
                        *final_gradient_mapping
                            .get(&input_coord)
                            .ok_or_else(|| EngineError::Other(String::new()))?,
                    ),
                    None,
                )
            };
            let base_colors = ColorPair::new(base_color, input_bg_color);
            let highlight_gradient = match &base_color {
                Some(base) => {
                    let highlight_color =
                        Animation::adjust_color_brightness(base, self.config.highlight_brightness);
                    Some(
                        Gradient::new(
                            &[*base, highlight_color, highlight_color, *base],
                            &[3, self.config.highlight_width, 3],
                            false,
                            false,
                        )
                        .map_err(EngineError::Other)?,
                    )
                }
                None => None,
            };
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation.set_appearance(
                    &input_symbol,
                    uses_pre,
                    Some(&input_symbol),
                    Some(base_colors),
                );
                ch.animation
                    .new_scene(false, None, None, "highlight", uses_pre);
                let scene = ch
                    .animation
                    .scenes
                    .get_mut("highlight")
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                if let Some(gradient) = &highlight_gradient {
                    for color in &gradient.spectrum {
                        scene
                            .add_frame(
                                &input_symbol,
                                2,
                                VisualParams {
                                    colors: Some(ColorPair::new(Some(*color), input_bg_color)),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                } else {
                    scene
                        .add_frame(
                            &input_symbol,
                            2,
                            VisualParams {
                                colors: Some(base_colors),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }
            ctx.terminal.set_character_visibility(id, true);
        }
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        let easer_complete = self.easer.as_ref()?.is_complete();
        if !ctx.active_characters.is_empty() || !easer_complete {
            let mut easer = self.easer.take()?;
            let step = easer.step();
            for group in step.added {
                for &id in group {
                    ctx.activate_scene(self, id, "highlight");
                    ctx.active_characters.insert(id);
                }
            }
            self.easer = Some(easer);
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            highlight_brightness: 1.75,
            highlight_direction: CharacterGroup::DiagonalBottomLeftToTopRight,
            highlight_width: 8,
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

impl Default for Highlight {
    fn default() -> Self {
        Self::new(HighlightConfig::default())
    }
}
