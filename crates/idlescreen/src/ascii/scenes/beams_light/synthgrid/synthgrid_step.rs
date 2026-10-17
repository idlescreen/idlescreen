// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CB_UPDATE_GROUP_TRACKER, CallbackValue, CharId, ColorPair, Coord, CoordColorMap, Direction,
    EffectCallback, EffectHooks, EngineCtx, EngineError, GridLine, HashMap, Phase, SynthGrid,
    SynthGridConfig, VisualParams, floor_div,
};

impl SynthGrid {
    pub fn new(config: SynthGridConfig) -> Self {
        SynthGrid {
            config,
            pending_groups: Vec::new(),
            grid_lines: Vec::new(),
            group_tracker: Vec::new(),
            character_final_color_map: HashMap::new(),
            phase: Phase::GridExpand,
            total_group_count: 0,
            active_groups: 0,
        }
    }

    /// SynthGridIterator.find_even_gap.
    pub(super) fn find_even_gap(dimension: i64) -> i64 {
        let dimension = dimension - 2;
        if dimension <= 0 {
            return 0;
        }
        // [i for i in range(dimension, 4, -1) if dimension % i <= 1]
        let mut potential_gaps: Vec<i64> = Vec::new();
        let mut i = dimension;
        while i > 4 {
            if dimension % i <= 1 {
                potential_gaps.push(i);
            }
            i -= 1;
        }
        if potential_gaps.is_empty() {
            return 4;
        }
        // min(potential_gaps, key=lambda x: abs(x - dimension // 5)) — first minimum wins
        let target = floor_div(dimension, 5);
        let mut best = potential_gaps[0];
        let mut best_key = (potential_gaps[0] - target).abs();
        for &gap in &potential_gaps[1..] {
            let key = (gap - target).abs();
            if key < best_key {
                best = gap;
                best_key = key;
            }
        }
        best
    }

    /// GridLine.__init__ (needs EffectHooks for activate_scene, so lives here).
    pub(super) fn make_grid_line(
        &mut self,
        ctx: &mut EngineCtx,
        origin: Coord,
        direction: Direction,
        grid_gradient_mapping: &CoordColorMap,
    ) -> Result<GridLine, EngineError> {
        let grid_symbol = match direction {
            Direction::Horizontal => self.config.grid_row_symbol.clone(),
            Direction::Vertical => self.config.grid_column_symbol.clone(),
        };
        let mut collapsed_characters: Vec<CharId> = Vec::new();
        let coords: Vec<Coord> = match direction {
            Direction::Horizontal => (ctx.terminal.canvas.left..=ctx.terminal.canvas.right)
                .map(|column_index| Coord::new(column_index, origin.row))
                .collect(),
            Direction::Vertical => (ctx.terminal.canvas.bottom..ctx.terminal.canvas.top)
                .map(|row_index| Coord::new(origin.column, row_index))
                .collect(),
        };
        for coord in coords {
            let effect_char = ctx.terminal.add_character(&grid_symbol, Coord::new(0, 0));
            let grid_scn = {
                let ch = &mut ctx.terminal.arena[effect_char.0 as usize];
                let uses_pre = ch.uses_input_preexisting_colors;
                let scene_id = ch.animation.new_scene(false, None, None, "", uses_pre);
                let fg = *grid_gradient_mapping
                    .get(&coord)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                ch.animation
                    .scenes
                    .get_mut(&scene_id)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .add_frame(
                        &grid_symbol,
                        1,
                        VisualParams {
                            colors: Some(ColorPair::new(Some(fg), None)),
                            ..Default::default()
                        },
                    )
                    .map_err(EngineError::Other)?;
                scene_id
            };
            ctx.activate_scene(self, effect_char, &grid_scn);
            let ch = &mut ctx.terminal.arena[effect_char.0 as usize];
            ch.layer = 2;
            ch.motion.set_coordinate(coord);
            collapsed_characters.push(effect_char);
        }
        Ok(GridLine {
            direction,
            collapsed_characters,
            extended_characters: Vec::new(),
        })
    }
}
impl EffectHooks for SynthGrid {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        callback: &EffectCallback,
    ) {
        if callback.id == CB_UPDATE_GROUP_TRACKER
            && let CallbackValue::Int(group_number) = callback.args[0]
        {
            self.group_tracker[group_number as usize] -= 1;
        }
    }
}
