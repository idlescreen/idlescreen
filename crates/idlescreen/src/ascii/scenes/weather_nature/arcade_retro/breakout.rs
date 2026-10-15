// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Autonomous retro Arkanoid/Breakout arcade playfield bouncing energy balls into banner bricks.

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
const PALETTE_STOPS: [&str; 5] = ["#00B4D8", "#00F5D4", "#FFBE0B", "#FB5607", "#FF006E"];
const FINAL_STOPS: [&str; 3] = ["#00F5D4", "#FFBE0B", "#FF006E"];

pub struct Breakout {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    balls: [(f64, f64, f64, f64); 3],
    frame: u32,
}

impl Breakout {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            balls: [
                (0.25, 0.82, 0.034, -0.048),
                (0.50, 0.78, -0.039, -0.044),
                (0.75, 0.84, -0.031, -0.051),
            ],
            frame: 0,
        }
    }

    fn step_balls(&mut self) {
        for b in &mut self.balls {
            b.0 += b.2;
            b.1 += b.3;
            if b.0 <= 0.05 || b.0 >= 0.95 {
                b.2 = -b.2;
                b.0 = b.0.clamp(0.05, 0.95);
            }
            if b.1 <= 0.08 || b.1 >= 0.88 {
                b.3 = -b.3;
                b.1 = b.1.clamp(0.08, 0.88);
            }
        }
    }
}

impl Default for Breakout {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Breakout {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Breakout {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.cells = build_canvas_cells(ctx, &FINAL_STOPS)?;
        self.palette = build_palette(&PALETTE_STOPS, 12)?;
        self.balls = [
            (0.25, 0.82, 0.034, -0.048),
            (0.50, 0.78, -0.039, -0.044),
            (0.75, 0.84, -0.031, -0.051),
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

        self.step_balls();
        let progress = self.frame as f64 / TOTAL_FRAMES as f64;
        let show_playfield = progress < 0.84;
        let paddle_x = self.balls[0].0.clamp(0.14, 0.86);
        let lock_line = 1.0 - progress * 1.15;

        for cell in &self.cells {
            let locked = !cell.is_fill && cell.ny >= lock_line;
            if locked {
                paint_symbol_cell(ctx, cell, &self.palette, None, 1.0, true);
                continue;
            }
            if !show_playfield {
                paint_symbol_cell(ctx, cell, &self.palette, None, 0.0, false);
                continue;
            }
            if (cell.ny - 0.91).abs() < 0.028 && (cell.nx - paddle_x).abs() < 0.11 {
                paint_symbol_cell(ctx, cell, &self.palette, Some("▀"), 0.15, false);
                continue;
            }
            let mut ball_hit = None;
            for (idx, &(bx, by, vx, vy)) in self.balls.iter().enumerate() {
                let d = ((cell.nx - bx) * 1.8).hypot(cell.ny - by);
                let td = ((cell.nx - (bx - vx)) * 1.8).hypot(cell.ny - (by - vy));
                if d < 0.055 {
                    ball_hit = Some(("█", 0.95 - idx as f64 * 0.2));
                    break;
                } else if td < 0.065 {
                    ball_hit = Some(("▒", 0.65 - idx as f64 * 0.15));
                }
            }
            if let Some((sym, col_t)) = ball_hit {
                paint_symbol_cell(ctx, cell, &self.palette, Some(sym), col_t, false);
            } else if !cell.is_fill {
                paint_symbol_cell(ctx, cell, &self.palette, Some("░"), cell.ny, false);
            } else {
                paint_symbol_cell(ctx, cell, &self.palette, None, 0.0, false);
            }
        }

        Some(emit_frame(ctx))
    }
}
