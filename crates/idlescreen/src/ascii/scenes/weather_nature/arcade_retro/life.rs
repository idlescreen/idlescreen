// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Conway's Game of Life cellular automata gliders and oscillators stabilizing into the banner.

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
const PALETTE_STOPS: [&str; 5] = ["#004B23", "#008000", "#38B000", "#70E000", "#CCFF33"];
const FINAL_STOPS: [&str; 3] = ["#38B000", "#70E000", "#CCFF33"];

pub struct Life {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    board: Vec<u8>,
    scratch: Vec<u8>,
    width: usize,
    height: usize,
    frame: u32,
}

impl Life {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            board: Vec::new(),
            scratch: Vec::new(),
            width: 80,
            height: 24,
            frame: 0,
        }
    }

    fn step_automata(&mut self) {
        let (w, h) = (self.width, self.height);
        if w == 0 || h == 0 {
            return;
        }
        for y in 0..h {
            let ym1 = (y + h - 1) % h;
            let yp1 = (y + 1) % h;
            for x in 0..w {
                let xm1 = (x + w - 1) % w;
                let xp1 = (x + 1) % w;
                let neighbors = [
                    (ym1, xm1),
                    (ym1, x),
                    (ym1, xp1),
                    (y, xm1),
                    (y, xp1),
                    (yp1, xm1),
                    (yp1, x),
                    (yp1, xp1),
                ]
                .iter()
                .filter(|&&(ny, nx)| self.board[ny * w + nx] >= 2)
                .count();
                let cur = self.board[y * w + x];
                let next = if neighbors == 3 || (cur >= 2 && neighbors == 2) {
                    (cur + 1).min(4)
                } else if cur >= 2 {
                    1
                } else {
                    0
                };
                self.scratch[y * w + x] = next;
            }
        }
        std::mem::swap(&mut self.board, &mut self.scratch);
    }
}

impl Default for Life {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Life {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Life {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.cells = build_canvas_cells(ctx, &FINAL_STOPS)?;
        self.palette = build_palette(&PALETTE_STOPS, 12)?;
        self.width = ctx.terminal.canvas.right.max(1) as usize;
        self.height = ctx.terminal.canvas.top.max(1) as usize;
        let total = self.width * self.height;
        self.board.clear();
        self.board.resize(total, 0);
        self.scratch.clear();
        self.scratch.resize(total, 0);
        for cell in &self.cells {
            let x = (cell.col - 1).max(0) as usize % self.width;
            let y = (cell.row - 1).max(0) as usize % self.height;
            let seed_hash = (x.wrapping_mul(37) ^ y.wrapping_mul(19)) % 7;
            if !cell.is_fill || seed_hash <= 1 {
                self.board[y * self.width + x] = 3;
            }
        }
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

        if self.frame % 2 == 1 {
            self.step_automata();
        }
        let progress = self.frame as f64 / TOTAL_FRAMES as f64;
        let show_automata = progress < 0.80;
        let reveal_r = progress * 1.85;
        let (w, h) = (self.width.max(1), self.height.max(1));

        for cell in &self.cells {
            let locked = !cell.is_fill && cell.dist_center <= reveal_r;
            if locked {
                paint_symbol_cell(ctx, cell, &self.palette, None, 1.0, true);
                continue;
            }
            if !show_automata {
                paint_symbol_cell(ctx, cell, &self.palette, None, 0.0, false);
                continue;
            }
            let x = (cell.col - 1).max(0) as usize % w;
            let y = (cell.row - 1).max(0) as usize % h;
            let state = self.board[y * w + x];
            let (sym, col_t) = match state {
                4 => (Some("█"), 0.95),
                3 => (Some("▓"), 0.75),
                2 => (Some("▒"), 0.50),
                1 => (Some("░"), 0.25),
                _ => (None, 0.0),
            };
            paint_symbol_cell(ctx, cell, &self.palette, sym, col_t, false);
        }

        Some(emit_frame(ctx))
    }
}
