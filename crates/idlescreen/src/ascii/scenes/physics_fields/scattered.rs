//! scattered, ported from effects/effect_scattered.py.

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
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

#[derive(Debug, Clone)]
pub struct ScatteredConfig {
    pub movement_speed: f64,
    pub movement_easing: Easing,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_frames: i64,
    pub final_gradient_direction: GradientDirection,
}

pub struct Scattered {
    config: ScatteredConfig,
    pending_chars: Vec<CharId>,
    character_final_color_map: HashMap<CharId, ColorPair>,
    initial_hold_frames: i64,
}

impl Scattered {
    pub fn new(config: ScatteredConfig) -> Self {
        Scattered {
            config,
            pending_chars: Vec::new(),
            character_final_color_map: HashMap::new(),
            initial_hold_frames: 0,
        }
    }
}

impl EffectHooks for Scattered {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Scattered {
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
            let start_coord = if ctx.terminal.canvas.right < 2 || ctx.terminal.canvas.top < 2 {
                Coord::new(1, 1)
            } else {
                ctx.terminal.canvas.random_coord(&mut ctx.rng, false, false)
            };
            let input_coord_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion.set_coordinate(start_coord);
                let path_id = ch
                    .motion
                    .new_path(
                        self.config.movement_speed,
                        Some(self.config.movement_easing),
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
            ctx.terminal.set_character_visibility(id, true);
            let gradient_scn = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .new_scene(false, Some(SyncMetric::Distance), None, "", uses_pre)
            };
            {
                let scene = {
                    let ch = &mut ctx.terminal.arena[id.0 as usize];
                    ch.animation
                        .scenes
                        .get_mut(&gradient_scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?
                };
                if dynamic {
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
                } else {
                    let final_fg_color = final_colors
                        .fg_color
                        .ok_or_else(|| EngineError::Other(String::new()))?;
                    let char_gradient = Gradient::with_steps(
                        &[final_gradient.spectrum[0], final_fg_color],
                        10,
                        false,
                    )
                    .map_err(EngineError::Other)?;
                    scene
                        .apply_gradient_to_symbols(
                            std::slice::from_ref(&input_symbol),
                            self.config.final_gradient_frames,
                            Some(&char_gradient),
                            None,
                        )
                        .map_err(EngineError::Other)?;
                }
            }
            ctx.activate_scene(self, id, &gradient_scn);
            ctx.active_characters.insert(id);
        }
        self.initial_hold_frames = 25;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.pending_chars.is_empty() || !ctx.active_characters.is_empty() {
            if self.initial_hold_frames != 0 {
                self.initial_hold_frames -= 1;
                return Some(emit_frame(ctx));
            }
            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for ScatteredConfig {
    fn default() -> Self {
        Self {
            movement_speed: 0.85,
            movement_easing: Easing::InOutBack,
            final_gradient_stops: vec![
                hex_color("ff9048"),
                hex_color("ab9dff"),
                hex_color("bdffea"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_frames: 3,
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}

impl Default for Scattered {
    fn default() -> Self {
        Self::new(ScatteredConfig::default())
    }
}
