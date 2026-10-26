// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Moiré interference: two counter-rotating line gratings whose beat
//! pattern sweeps across the canvas before settling into the banner.
//!
//! The effect is a pure function of position and time, so it costs one
//! cosine per grating per cell and allocates nothing per frame.

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
const PALETTE_STOPS: [&str; 5] = ["#0B0B2E", "#3B1E6E", "#B5338C", "#F26B5B", "#FFE9A3"];
const FINAL_STOPS: [&str; 3] = ["#B5338C", "#F26B5B", "#FFE9A3"];

/// Line density of the first grating (cycles across the canvas).
const GRIDING_A: f64 = 21.0;
/// Line density of the second grating.
const GRIDING_B: f64 = 17.0;
/// Angular speed of the first grating, radians per frame.
const SPIN_A: f64 = 0.026;
/// Angular speed of the second grating, radians per frame.
const SPIN_B: f64 = -0.017;
/// Angle between the two gratings at the start of the pass.
const SKEW: f64 = 0.62;

pub struct Moire {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    frame: u32,
}

impl Moire {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for Moire {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Moire {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Moire {
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
        let angle_a = self.frame as f64 * SPIN_A;
        let angle_b = self.frame as f64 * SPIN_B + SKEW;
        let (sin_a, cos_a) = angle_a.sin_cos();
        let (sin_b, cos_b) = angle_b.sin_cos();
        // The fringes fade in, hold, then fade out into the settled banner.
        let fill_fade = if progress > 0.78 {
            ((1.0 - progress) / 0.22).clamp(0.0, 1.0)
        } else {
            1.0
        };

        for cell in &self.cells {
            // Project the cell onto each grating's normal; the product of the
            // two cosines is the classic two-source interference term.
            let ua = cell.dx * cos_a + cell.dy * sin_a;
            let ub = cell.dx * cos_b + cell.dy * sin_b;
            let beat = (ua * GRIDING_A).cos() * (ub * GRIDING_B).cos();
            // Radial falloff keeps the fringes inside the canvas and lets the
            // corners settle early.
            let falloff = (1.0 - cell.dist_center * 0.45).clamp(0.0, 1.0);
            let intensity = ((0.5 + 0.5 * beat) * falloff).clamp(0.0, 1.0);
            paint_math_cell(ctx, cell, &self.palette, intensity, false, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
