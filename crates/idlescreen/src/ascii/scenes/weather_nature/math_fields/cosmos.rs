// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Deep-space logarithmic spiral galaxy and starfield effect settling into the banner.

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
const PALETTE_STOPS: [&str; 5] = ["#140A38", "#3C1B7A", "#8E24AA", "#00D8FF", "#FFF8E7"];
const FINAL_STOPS: [&str; 3] = ["#8E24AA", "#00D8FF", "#FFF8E7"];

pub struct Cosmos {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    frame: u32,
}

impl Cosmos {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for Cosmos {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Cosmos {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Cosmos {
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
        let t = self.frame as f64 * 0.08;
        let fill_fade = if progress > 0.72 {
            ((1.0 - progress) / 0.28).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let reveal_r = progress * 1.85;

        for cell in &self.cells {
            let r = cell.dist_center;
            let arm_angle = 2.0 * (cell.angle_center - t * 1.5) + 6.8 * r;
            let arm = (0.5 + 0.5 * arm_angle.cos()).powi(3) * (-r * 1.15).exp();
            let core = (-r * 3.8).exp();
            let star_hash = ((cell.col as u64).wrapping_mul(73856093)
                ^ (cell.row as u64).wrapping_mul(19349663))
                % 97;
            let star = if star_hash == 0 {
                0.55 + 0.45 * (t * 4.0 + cell.col as f64).sin()
            } else {
                0.0
            };
            let intensity = (arm * 0.85 + core * 0.95).max(star).clamp(0.0, 1.0);
            let locked = !cell.is_fill && r <= reveal_r;
            paint_math_cell(ctx, cell, &self.palette, intensity, locked, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
