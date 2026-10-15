// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Dual-source 2D radial wave interference ripple effect settling into the banner.

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
const PALETTE_STOPS: [&str; 5] = ["#031A38", "#00509D", "#0096C7", "#48CAE4", "#CAF0F8"];
const FINAL_STOPS: [&str; 3] = ["#0096C7", "#48CAE4", "#CAF0F8"];

pub struct Ripple {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    frame: u32,
}

impl Ripple {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for Ripple {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Ripple {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Ripple {
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
        let t = self.frame as f64 * 0.11;
        let s1 = ((t * 1.4).cos() * 0.45, (t * 1.4).sin() * 0.30);
        let s2 = (-(t * 1.7).cos() * 0.45, -(t * 1.7).sin() * 0.30);
        let fill_fade = if progress > 0.72 {
            ((1.0 - progress) / 0.28).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let reveal_r = progress * 1.85;

        for cell in &self.cells {
            let d1 = (cell.dx - s1.0).hypot(cell.dy - s1.1);
            let d2 = (cell.dx - s2.0).hypot(cell.dy - s2.1);
            let w1 = (d1 * 18.0 - t * 4.5).sin();
            let w2 = (d2 * 18.0 - t * 4.5).sin();
            let atten = (-0.55 * d1.min(d2)).exp();
            let intensity = ((0.5 + 0.25 * (w1 + w2)) * atten).clamp(0.0, 1.0);
            let locked = !cell.is_fill && cell.dist_center <= reveal_r;
            paint_math_cell(ctx, cell, &self.palette, intensity, locked, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
