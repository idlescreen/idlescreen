// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    Bubbles, BubblesConfig, CallerKey, CharId, CharacterFilter, CharacterGroup, CharacterSort,
    Color, ColorPair, Coord, Easing, Effect, EngineCtx, EngineError, Event, EventAction,
    ExistingColorHandling, Gradient, VisualParams, emit_frame,
};

impl Bubbles {
    pub(super) fn build_scene(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
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
            let (input_coord, input_symbol, input_fg, input_bg, uses_pre) = {
                let ch = &ctx.terminal.arena[id.0 as usize];
                (
                    ch.input_coord,
                    ch.input_symbol.clone(),
                    ch.animation.input_fg_color,
                    ch.animation.input_bg_color,
                    ch.uses_input_preexisting_colors,
                )
            };
            self.character_final_color_map.insert(
                id,
                *final_gradient_mapping
                    .get(&input_coord)
                    .ok_or_else(|| EngineError::Other(String::new()))?,
            );
            ctx.terminal.arena[id.0 as usize].layer = 1;
            let (pop_1_scene, pop_2_scene, final_scene) = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let pop_1 = ch.animation.new_scene(false, None, None, "pop_1", uses_pre);
                let pop_2 = ch.animation.new_scene(false, None, None, "", uses_pre);
                for (scn, sym) in [(&pop_1, "*"), (&pop_2, "'")] {
                    ch.animation
                        .scenes
                        .get_mut(scn)
                        .ok_or_else(|| EngineError::Other(String::new()))?
                        .add_frame(
                            sym,
                            9,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(self.config.pop_color), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
                let final_s = ch.animation.new_scene(false, None, None, "", uses_pre);
                (pop_1, pop_2, final_s)
            };
            if dynamic {
                let step_grad = |opt: &Option<Color>| {
                    opt.as_ref()
                        .map(|c| {
                            Gradient::with_steps(&[self.config.pop_color, *c], 8, false)
                                .map_err(EngineError::Other)
                        })
                        .transpose()
                };
                let fg_gradient = step_grad(&input_fg)?;
                let bg_gradient = step_grad(&input_bg)?;
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
                            6,
                            fg_gradient.as_ref(),
                            bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    scene
                        .add_frame(
                            &input_symbol,
                            6,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            } else {
                let final_color = *self
                    .character_final_color_map
                    .get(&id)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                let char_final_gradient =
                    Gradient::with_steps(&[self.config.pop_color, final_color], 8, false)
                        .map_err(EngineError::Other)?;
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.animation
                    .scenes
                    .get_mut(&final_scene)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .apply_gradient_to_symbols(
                        std::slice::from_ref(&input_symbol),
                        6,
                        Some(&char_final_gradient),
                        None,
                    )
                    .map_err(EngineError::Other)?;
            }
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene(pop_1_scene),
                EventAction::ActivateScene(pop_2_scene.clone()),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::SceneComplete,
                CallerKey::Scene(pop_2_scene),
                EventAction::ActivateScene(final_scene),
            )
            .map_err(EngineError::Other)?;
            let final_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let path_id = ch
                    .motion
                    .new_path(0.3, Some(Easing::InOutExpo), None, 0, false, "final")
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
                Event::PathComplete,
                CallerKey::Path(final_path),
                EventAction::SetLayer(0),
            )
            .map_err(EngineError::Other)?;
        }

        let mut unbubbled_chars: Vec<CharId> = Vec::new();
        for char_list in ctx
            .terminal
            .get_characters_grouped(CharacterFilter::default(), CharacterGroup::RowBottomToTop)
        {
            unbubbled_chars.extend(char_list);
        }
        self.bubbles = Vec::new();
        while !unbubbled_chars.is_empty() {
            let mut bubble_group: Vec<CharId> = Vec::new();
            if unbubbled_chars.len() < 5 {
                bubble_group.append(&mut unbubbled_chars);
            } else {
                let count = ctx
                    .rng
                    .randint(5, std::cmp::min(unbubbled_chars.len() as i64, 20));
                for _ in 0..count {
                    bubble_group.push(unbubbled_chars.remove(0));
                }
            }
            let bubble_origin = Coord::new(
                ctx.rng
                    .randint(ctx.terminal.canvas.left, ctx.terminal.canvas.right),
                ctx.terminal.canvas.top + 10,
            );
            let new_bubble = self.make_bubble(ctx, bubble_origin, bubble_group)?;
            self.bubbles.push(new_bubble);
        }
        self.animating_bubbles = Vec::new();
        self.steps_since_last_bubble = 0;
        Ok(())
    }
}
impl Effect for Bubbles {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.build_scene(ctx)
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if !self.animating_bubbles.is_empty()
            || !ctx.active_characters.is_empty()
            || !self.bubbles.is_empty()
        {
            if !self.bubbles.is_empty() && self.steps_since_last_bubble >= self.config.bubble_delay
            {
                let next_bubble = self.bubbles.remove(0);
                for &id in &next_bubble.characters {
                    ctx.terminal.set_character_visibility(id, true);
                }
                self.animating_bubbles.push(next_bubble);
                self.steps_since_last_bubble = 0;
            }
            self.steps_since_last_bubble += 1;

            let mut animating = std::mem::take(&mut self.animating_bubbles);
            for bubble in &animating {
                if bubble.landed {
                    self.bubble_pop(ctx, bubble);
                    for &id in &bubble.characters {
                        ctx.active_characters.insert(id);
                    }
                }
            }
            animating.retain(|bubble| !bubble.landed);
            for bubble in &mut animating {
                self.bubble_move(ctx, bubble);
            }
            self.animating_bubbles = animating;

            ctx.update(self);
            return Some(emit_frame(ctx));
        }
        None
    }
}

impl Default for Bubbles {
    fn default() -> Self {
        Self::new(BubblesConfig::default())
    }
}
