// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Amiga demoscene multi-frequency trigonometric plasma & copper raster bars scene.

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
const PALETTE_STOPS: [&str; 6] = [
    "#240046", "#7B2CBF", "#FF007F", "#FF8800", "#FFEA00", "#00F5D4",
];
const FINAL_STOPS: [&str; 3] = ["#FF007F", "#FFEA00", "#00F5D4"];

pub struct Plasma {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    frame: u32,
}

impl Plasma {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for Plasma {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Plasma {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Plasma {
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
        let t = self.frame as f64 * 0.11;
        let fill_fade = if progress > 0.72 {
            ((1.0 - progress) / 0.28).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let reveal_x = (progress * 1.35 - 0.15).clamp(0.0, 1.0);

        for cell in &self.cells {
            let v1 = (cell.dx * 4.5 + t).sin();
            let v2 = (cell.dy * 5.2 - t * 1.2).sin();
            let v3 = ((cell.dx + cell.dy) * 4.0 + t * 0.8).sin();
            let v4 = (cell.dist_center * 7.5 - t * 1.6).sin();
            let copper = (cell.ny * 14.0 + t * 2.2).sin() * 0.2;
            let raw = (v1 + v2 + v3 + v4) * 0.125 + 0.5 + copper;
            let intensity = raw.clamp(0.15, 1.0);
            let locked = !cell.is_fill && cell.nx <= reveal_x;
            paint_math_cell(ctx, cell, &self.palette, intensity, locked, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
