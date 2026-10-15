// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Falling 16-bit tetromino pixel block stacker with line-clear flashes assembling the banner.

use crate::ascii::scenes::canvas_ops::emit_frame;
use crate::ascii::scenes::weather_nature::field_ops::{
    CanvasCell, build_canvas_cells, build_palette, paint_symbol_cell, settle_canvas_cells,
};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::utils::graphics::Color;

const TOTAL_FRAMES: u32 = 64;
const PALETTE_STOPS: [&str; 6] = [
    "#00F5D4", "#3A86FF", "#8338EC", "#FF006E", "#FB5607", "#FFBE0B",
];
const FINAL_STOPS: [&str; 3] = ["#00F5D4", "#FF006E", "#FFBE0B"];

pub struct Tetris {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    frame: u32,
}

impl Tetris {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for Tetris {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Tetris {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Tetris {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.cells = build_canvas_cells(ctx, &FINAL_STOPS)?;
        self.palette = build_palette(&PALETTE_STOPS, 10)?;
        self.frame = 0;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.frame >= TOTAL_FRAMES {
            return None;
        }
        self.frame += 1;
        if self.frame == TOTAL_FRAMES {
            settle_canvas_cells(ctx, &self.cells);
            return Some(emit_frame(ctx));
        }

        let progress = self.frame as f64 / TOTAL_FRAMES as f64;
        let show_falling = progress < 0.82;

        for cell in &self.cells {
            let piece_col = (cell.col / 2) as u64;
            let col_phase = ((piece_col.wrapping_mul(2654435761)) % 100) as f64 / 100.0;
            let col_color = ((piece_col.wrapping_mul(17)) % 6) as f64 / 5.0;
            let lock_threshold = 1.0 - (progress * 1.35 - col_phase * 0.28).clamp(0.0, 1.0);
            let locked = !cell.is_fill && cell.ny >= lock_threshold;
            if locked {
                paint_symbol_cell(ctx, cell, &self.palette, None, 1.0, true);
                continue;
            }
            if !show_falling {
                paint_symbol_cell(ctx, cell, &self.palette, None, 0.0, false);
                continue;
            }
            let drop_y = (progress * 1.8 + col_phase).fract();
            let dist_drop = (cell.ny - drop_y).abs();
            if dist_drop < 0.065 && (cell.col + cell.row) % 2 == 0 {
                let sym = if dist_drop < 0.03 { "█" } else { "▓" };
                paint_symbol_cell(ctx, cell, &self.palette, Some(sym), col_color, false);
            } else if !cell.is_fill && (cell.ny - lock_threshold).abs() < 0.05 {
                paint_symbol_cell(ctx, cell, &self.palette, Some("▒"), 0.95, false);
            } else {
                paint_symbol_cell(ctx, cell, &self.palette, None, 0.0, false);
            }
        }

        Some(emit_frame(ctx))
    }
}
