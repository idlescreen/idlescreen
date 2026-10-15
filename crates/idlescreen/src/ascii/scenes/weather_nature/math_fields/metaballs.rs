// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Glowing molten lava-lamp isosurface metaballs effect settling into the 16-bit banner.

use crate::ascii::scenes::canvas_ops::emit_frame;
use crate::ascii::scenes::weather_nature::field_ops::{
    CanvasCell, build_canvas_cells, build_palette, paint_math_cell, settle_canvas_cells,
};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::utils::graphics::Color;

const TOTAL_FRAMES: u32 = 64;
const PALETTE_STOPS: [&str; 5] = ["#3A0CA3", "#7209B7", "#F72585", "#FF6D00", "#FFEA00"];
const FINAL_STOPS: [&str; 3] = ["#F72585", "#FF6D00", "#FFEA00"];

pub struct Metaballs {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    frame: u32,
}

impl Metaballs {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for Metaballs {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Metaballs {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Metaballs {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.cells = build_canvas_cells(ctx, &FINAL_STOPS)?;
        self.palette = build_palette(&PALETTE_STOPS, 12)?;
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
        let t = self.frame as f64 * 0.085;
        let converge = (1.0 - progress * 0.65).max(0.2);
        let blobs = [
            (
                (t * 1.3).cos() * 0.75 * converge,
                (t * 1.9).sin() * 0.55 * converge,
                0.22,
            ),
            (
                -(t * 1.7).sin() * 0.80 * converge,
                (t * 1.1).cos() * 0.50 * converge,
                0.24,
            ),
            (
                (t * 2.1).sin() * 0.55 * converge,
                -(t * 1.5).cos() * 0.60 * converge,
                0.20,
            ),
            (
                -(t * 1.2).cos() * 0.65 * converge,
                -(t * 2.2).sin() * 0.45 * converge,
                0.21,
            ),
        ];
        let fill_fade = if progress > 0.72 {
            ((1.0 - progress) / 0.28).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let reveal_r = progress * 1.85;

        for cell in &self.cells {
            let mut sum = 0.0f64;
            for &(bx, by, r2) in &blobs {
                let d2 = (cell.dx - bx).powi(2) + (cell.dy - by).powi(2) + 0.035;
                sum += r2 / d2;
            }
            let intensity = ((sum - 0.85) / 2.4).clamp(0.0, 1.0);
            let locked = !cell.is_fill && cell.dist_center <= reveal_r;
            paint_math_cell(ctx, cell, &self.palette, intensity, locked, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
