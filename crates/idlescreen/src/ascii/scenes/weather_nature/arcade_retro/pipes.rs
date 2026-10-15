// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! High-speed 16-bit neon pipes racing across a Manhattan grid to scaffold the banner.

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
const PALETTE_STOPS: [&str; 5] = ["#00F5D4", "#00BBF9", "#9B5DE5", "#F15BB5", "#FEE440"];
const FINAL_STOPS: [&str; 3] = ["#00F5D4", "#F15BB5", "#FEE440"];

struct PipeHead {
    x: i64,
    y: i64,
    dx: i64,
    dy: i64,
    color_idx: u8,
}

pub struct Pipes {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    grid: Vec<u8>,
    heads: Vec<PipeHead>,
    width: i64,
    height: i64,
    frame: u32,
}

impl Pipes {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            grid: Vec::new(),
            heads: Vec::new(),
            width: 80,
            height: 24,
            frame: 0,
        }
    }

    fn step_pipes(&mut self, tick: u64) {
        let w = self.width.max(1);
        let h = self.height.max(1);
        for (i, head) in self.heads.iter_mut().enumerate() {
            for s in 0..3 {
                head.x = (head.x + head.dx).rem_euclid(w);
                head.y = (head.y + head.dy).rem_euclid(h);
                let idx = (head.y * w + head.x) as usize;
                if idx < self.grid.len() {
                    self.grid[idx] = head.color_idx;
                }
                let turn_hash = (tick.wrapping_add(s)).wrapping_mul(31 + i as u64 * 17);
                if turn_hash % 5 == 0 {
                    if head.dx != 0 {
                        head.dx = 0;
                        head.dy = if turn_hash % 2 == 0 { 1 } else { -1 };
                    } else {
                        head.dy = 0;
                        head.dx = if turn_hash % 2 == 0 { 1 } else { -1 };
                    }
                }
            }
        }
    }
}

impl Default for Pipes {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Pipes {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Pipes {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.cells = build_canvas_cells(ctx, &FINAL_STOPS)?;
        self.palette = build_palette(&PALETTE_STOPS, 12)?;
        self.width = ctx.terminal.canvas.right.max(1);
        self.height = ctx.terminal.canvas.top.max(1);
        self.grid.clear();
        self.grid.resize((self.width * self.height) as usize, 0);
        self.heads = vec![
            PipeHead {
                x: self.width / 4,
                y: self.height / 2,
                dx: 1,
                dy: 0,
                color_idx: 1,
            },
            PipeHead {
                x: (self.width * 3) / 4,
                y: self.height / 2,
                dx: -1,
                dy: 0,
                color_idx: 2,
            },
            PipeHead {
                x: self.width / 2,
                y: self.height / 3,
                dx: 0,
                dy: 1,
                color_idx: 3,
            },
            PipeHead {
                x: self.width / 2,
                y: (self.height * 2) / 3,
                dx: 0,
                dy: -1,
                color_idx: 4,
            },
        ];
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

        self.step_pipes(self.frame as u64);
        let progress = self.frame as f64 / TOTAL_FRAMES as f64;
        let show_pipes = progress < 0.80;
        let reveal_r = progress * 1.85;
        let w = self.width.max(1);

        for cell in &self.cells {
            let locked = !cell.is_fill && cell.dist_center <= reveal_r;
            if locked {
                paint_symbol_cell(ctx, cell, &self.palette, None, 1.0, true);
                continue;
            }
            if !show_pipes {
                paint_symbol_cell(ctx, cell, &self.palette, None, 0.0, false);
                continue;
            }
            let gx = (cell.col - 1).clamp(0, w - 1);
            let gy = (cell.row - 1).clamp(0, self.height - 1);
            let val = self.grid.get((gy * w + gx) as usize).copied().unwrap_or(0);
            if val > 0 {
                let sym = if gy % 2 == 0 { "▀" } else { "█" };
                let col_t = (val as f64 - 1.0) / 3.0;
                paint_symbol_cell(ctx, cell, &self.palette, Some(sym), col_t, false);
            } else {
                paint_symbol_cell(ctx, cell, &self.palette, None, 0.0, false);
            }
        }

        Some(emit_frame(ctx))
    }
}
