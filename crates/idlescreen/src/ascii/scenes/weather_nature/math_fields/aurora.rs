// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Boreal aurora harmonic ribbon curtain effect settling into the 16-bit beveled banner.

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
const PALETTE_STOPS: [&str; 5] = ["#003B24", "#00E575", "#00D8FF", "#7A28FF", "#E0B0FF"];
const FINAL_STOPS: [&str; 3] = ["#00E575", "#00D8FF", "#E0B0FF"];

pub struct Aurora {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    frame: u32,
}

impl Aurora {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for Aurora {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Aurora {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Aurora {
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
        let t = self.frame as f64 * 0.09;
        let fill_fade = if progress > 0.72 {
            ((1.0 - progress) / 0.28).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let reveal_y = (progress * 1.35 - 0.15).clamp(0.0, 1.0);

        for cell in &self.cells {
            let ribbon1 = 0.38
                + 0.20 * (cell.nx * 5.2 + t * 1.9).sin()
                + 0.10 * (cell.nx * 11.5 - t * 1.3).cos();
            let ribbon2 = 0.64
                + 0.18 * (cell.nx * 4.1 - t * 1.6).cos()
                + 0.09 * ((cell.nx + cell.ny) * 8.4 + t * 2.3).sin();
            let d1 = (cell.ny - ribbon1).abs();
            let d2 = (cell.ny - ribbon2).abs();
            let curtain = (-d1 * 6.0).exp().max((-d2 * 6.5).exp() * 0.88);
            let shimmer = 0.55 + 0.45 * (cell.nx * 18.0 + t * 3.4).sin();
            let intensity = (curtain * shimmer).clamp(0.0, 1.0);
            let locked = !cell.is_fill && cell.ny <= reveal_y;
            paint_math_cell(ctx, cell, &self.palette, intensity, locked, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
