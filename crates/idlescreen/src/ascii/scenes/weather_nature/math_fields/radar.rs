// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Polar CRT phosphor radar sweep and range-ring ping effect settling into the banner.

use crate::ascii::scenes::canvas_ops::emit_frame;
use crate::ascii::scenes::weather_nature::field_ops::{
    CanvasCell, build_canvas_cells, build_palette, paint_math_cell, settle_canvas_cells,
};
use std::f64::consts::TAU;
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::utils::graphics::Color;

const TOTAL_FRAMES: u32 = 64;
const PALETTE_STOPS: [&str; 5] = ["#00220B", "#005C1F", "#00B844", "#00FF66", "#D4FFE4"];
const FINAL_STOPS: [&str; 3] = ["#00B844", "#00FF66", "#D4FFE4"];

pub struct Radar {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    frame: u32,
}

impl Radar {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for Radar {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Radar {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Radar {
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
        let beam_angle = (progress * TAU * 1.65) - std::f64::consts::PI;
        let fill_fade = if progress > 0.72 {
            ((1.0 - progress) / 0.28).clamp(0.0, 1.0)
        } else {
            1.0
        };

        for cell in &self.cells {
            let r = cell.dist_center;
            if r > 1.38 {
                let locked = !cell.is_fill && progress > 0.65;
                paint_math_cell(ctx, cell, &self.palette, 0.0, locked, fill_fade);
                continue;
            }
            let delta = (beam_angle - cell.angle_center).rem_euclid(TAU);
            let sweep = (-delta * 1.9).exp();
            let ring = if (r * 16.0).sin().abs() > 0.90 {
                0.36
            } else {
                0.0
            };
            let crosshair = if cell.dx.abs() < 0.03 || cell.dy.abs() < 0.03 {
                0.28
            } else {
                0.0
            };
            let intensity = (sweep * 0.95 + ring + crosshair).clamp(0.0, 1.0);
            let cell_norm_angle = (cell.angle_center + std::f64::consts::PI) / TAU;
            let locked = !cell.is_fill && (progress * 1.35 >= cell_norm_angle);
            paint_math_cell(ctx, cell, &self.palette, intensity, locked, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
