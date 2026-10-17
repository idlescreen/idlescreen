//! expand, ported from effects/effect_expand.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, SyncMetric, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

#[derive(Debug, Clone)]
pub struct ExpandConfig {
    pub expand_easing: Easing,
    pub movement_speed: f64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}

pub struct Expand {
    config: ExpandConfig,
    character_final_color_map: HashMap<CharId, ColorPair>,
}

impl Expand {
    pub fn new(config: ExpandConfig) -> Self {
        Expand {
            config,
            character_final_color_map: HashMap::new(),
        }
    }
}

impl EffectHooks for Expand {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Expand {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
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
        let characters = ctx.terminal.get_characters(
            &mut ctx.rng,
            CharacterFilter::default(),
            CharacterSort::TopToBottomLeftToRight,
        );
        for id in characters {
            let (input_fg, input_bg, input_coord) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.animation.input_fg_color,
                    ch.animation.input_bg_color,
                    ch.input_coord,
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
        }
        let characters = ctx.terminal.get_characters(
            &mut ctx.rng,
            CharacterFilter::default(),
            CharacterSort::TopToBottomLeftToRight,
        );
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
            let center = ctx.terminal.canvas.center;
            let input_coord_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion.set_coordinate(center);
                let path_id = ch
                    .motion
                    .new_path(
                        self.config.movement_speed,
                        Some(self.config.expand_easing),
                        None,
                        0,
                        false,
                        "",
                    )
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&path_id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(input_coord, None, "")
                    .map_err(EngineError::Other)?;
                path_id
            };
            ctx.terminal.set_character_visibility(id, true);
            ctx.active_characters.insert(id);
            ctx.register_event(
                id,
                Event::PathActivated,
                CallerKey::Path(input_coord_path.clone()),
                EventAction::SetLayer(1),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(input_coord_path.clone()),
                EventAction::SetLayer(0),
            )
            .map_err(EngineError::Other)?;
            ctx.activate_path(self, id, &input_coord_path);
            let gradient_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .new_scene(false, Some(SyncMetric::Distance), None, "", uses_pre)
            };
            let symbols = vec![input_symbol.clone()];
            {
                let scene = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation
                        .scenes
                        .get_mut(&gradient_scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?
                };
                if dynamic {
                    let step_grad = |opt: &Option<Color>| {
                        opt.as_ref()
                            .map(|c| {
                                Gradient::with_steps(&[final_gradient.spectrum[0], *c], 10, false)
                                    .map_err(EngineError::Other)
                            })
                            .transpose()
                    };
                    let fg_gradient = step_grad(&input_fg)?;
                    let bg_gradient = step_grad(&input_bg)?;
                    if fg_gradient.is_some() || bg_gradient.is_some() {
                        scene
                            .apply_gradient_to_symbols(
                                &symbols,
                                1,
                                fg_gradient.as_ref(),
                                bg_gradient.as_ref(),
                            )
                            .map_err(EngineError::Other)?;
                    } else {
                        scene
                            .add_frame(
                                &input_symbol,
                                1,
                                VisualParams {
                                    colors: Some(ColorPair::default()),
                                    ..Default::default()
                                },
                            )
                            .map_err(EngineError::Other)?;
                    }
                } else {
                    let final_fg = self.character_final_color_map[&id]
                        .fg_color
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    let gradient =
                        Gradient::with_steps(&[final_gradient.spectrum[0], final_fg], 10, false)
                            .map_err(EngineError::Other)?;
                    scene
                        .apply_gradient_to_symbols(&symbols, 5, Some(&gradient), None)
                        .map_err(EngineError::Other)?;
                }
            }
            ctx.activate_scene(self, id, &gradient_scn);
        }
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !ctx.active_characters.is_empty() {
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for ExpandConfig {
    fn default() -> Self {
        Self {
            expand_easing: Easing::InOutQuart,
            movement_speed: 0.35,
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

impl Default for Expand {
    fn default() -> Self {
        Self::new(ExpandConfig::default())
    }
}
