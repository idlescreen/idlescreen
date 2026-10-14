// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

impl Matrix {
    pub fn new(config: MatrixConfig) -> Self {
        let resolve_delay = config.resolve_delay;
        Matrix {
            config,
            columns: Vec::new(),
            pending_columns: Vec::new(),
            active_columns: Vec::new(),
            full_columns: Vec::new(),
            character_final_color_map: HashMap::new(),
            rain_colors: Vec::new(),
            column_delay: 0,
            resolve_delay,
            final_frame_shown: false,
            rain_complete: false,
            phase: Phase::Rain,
            rain_start: 0.0,
        }
    }

    /// MatrixIterator._has_input_colors.
    pub(super) fn has_input_colors(ctx: &EngineCtx, character: CharId) -> bool {
        let anim = &ctx.terminal.arena[character.0 as usize].animation;
        anim.input_fg_color.is_some() || anim.input_bg_color.is_some()
    }

    pub(super) fn column_phase_for(&self) -> ColumnPhase {
        match self.phase {
            Phase::Rain => ColumnPhase::Rain,
            Phase::Fill => ColumnPhase::Fill,
            Phase::Resolve => unreachable!("columns are never set up during resolve"),
        }
    }
}
impl EffectHooks for Matrix {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
impl Matrix {
    pub(super) fn build_scene(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        // __init__: rain_colors gradient (no RNG consumed)
        self.rain_colors = Gradient::with_steps(&self.config.rain_color_gradient, 6, false)
            .map_err(EngineError::Other)?
            .spectrum;

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
        for character in characters {
            let (input_symbol, input_coord, input_fg, input_bg, uses_pre) = {
                let ch = &ctx.terminal.arena[character.0 as usize];
                (
                    ch.input_symbol.clone(),
                    ch.input_coord,
                    ch.animation.input_fg_color.clone(),
                    ch.animation.input_bg_color.clone(),
                    ch.uses_input_preexisting_colors,
                )
            };
            let final_colors = if dynamic {
                ColorPair::new(input_fg, input_bg)
            } else {
                ColorPair::new(
                    Some(final_gradient_mapping.get(&input_coord).must_val().clone()),
                    None,
                )
            };
            self.character_final_color_map
                .insert(character, final_colors.clone());
            let final_fg_color = final_colors.fg_color.clone();
            let final_bg_color = final_colors.bg_color.clone();
            let resolve_scn = ctx.terminal.arena[character.0 as usize]
                .animation
                .new_scene(false, None, None, "resolve", uses_pre);
            if dynamic {
                let fg_gradient = match &final_fg_color {
                    Some(fg) => Some(
                        Gradient::with_steps(
                            &[self.config.highlight_color.clone(), fg.clone()],
                            8,
                            false,
                        )
                        .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let bg_gradient = match &final_bg_color {
                    Some(bg) => Some(
                        Gradient::with_steps(
                            &[self.config.highlight_color.clone(), bg.clone()],
                            8,
                            false,
                        )
                        .map_err(EngineError::Other)?,
                    ),
                    None => None,
                };
                let scene = ctx.terminal.arena[character.0 as usize]
                    .animation
                    .scenes
                    .get_mut(&resolve_scn)
                    .must_val();
                if fg_gradient.is_some() || bg_gradient.is_some() {
                    scene
                        .apply_gradient_to_symbols(
                            &[input_symbol.clone()],
                            self.config.final_gradient_frames,
                            fg_gradient.as_ref(),
                            bg_gradient.as_ref(),
                        )
                        .map_err(EngineError::Other)?;
                } else {
                    scene
                        .add_frame(
                            &input_symbol,
                            self.config.final_gradient_frames,
                            VisualParams {
                                colors: Some(ColorPair::default()),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            } else {
                let final_fg_color = final_fg_color.must_val();
                let resolve_gradient = Gradient::with_steps(
                    &[self.config.highlight_color.clone(), final_fg_color],
                    8,
                    false,
                )
                .map_err(EngineError::Other)?;
                for color in &resolve_gradient.spectrum {
                    ctx.terminal.arena[character.0 as usize]
                        .animation
                        .scenes
                        .get_mut(&resolve_scn)
                        .must_val()
                        .add_frame(
                            &input_symbol,
                            self.config.final_gradient_frames,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(color.clone()), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }
        }

        let all_chars_filter = CharacterFilter {
            input_chars: true,
            inner_fill_chars: true,
            outer_fill_chars: true,
            added_chars: false,
        };
        for mut column_chars in ctx
            .terminal
            .get_characters_grouped(all_chars_filter, CharacterGroup::ColumnLeftToRight)
        {
            column_chars.reverse();
            let column = RainColumn::new(ctx, &self.config, column_chars);
            self.columns.push(column);
            self.pending_columns.push(self.columns.len() - 1);
        }
        ctx.rng.shuffle(&mut self.pending_columns);

        // MatrixIterator.__init__: rain_start = time.time() (after build).
        self.rain_start = ctx.clock.now_wall();
        Ok(())
    }
}
