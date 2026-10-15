// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 3D polar wormhole/vortex tunnel banking through neon checkerboard ring segments.

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
const PALETTE_STOPS: [&str; 5] = ["#12002B", "#3F0071", "#FB2576", "#00F5D4", "#FFF8E7"];
const FINAL_STOPS: [&str; 3] = ["#FB2576", "#00F5D4", "#FFF8E7"];

pub struct Tunnel {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    frame: u32,
}

impl Tunnel {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for Tunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Tunnel {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Tunnel {
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
        let t = self.frame as f64 * 0.10;
        let bank_x = (t * 1.4).cos() * 0.22 * (1.0 - progress);
        let bank_y = (t * 1.9).sin() * 0.15 * (1.0 - progress);
        let fill_fade = if progress > 0.72 {
            ((1.0 - progress) / 0.28).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let reveal_r = progress * 1.85;

        for cell in &self.cells {
            let rx = cell.dx - bank_x;
            let ry = cell.dy - bank_y;
            let r = rx.hypot(ry).max(0.06);
            let angle = ry.atan2(rx);
            let u = (angle / TAU + 0.5) * 12.0 + (0.35 / r) * (t * 0.8).sin();
            let v = 1.35 / r - t * 3.8;
            let checker = ((u.floor() as i64).wrapping_add(v.floor() as i64)).rem_euclid(2);
            let ring_glow = (v * std::f64::consts::PI).sin().abs();
            let depth_fog = (r * 0.95).clamp(0.15, 1.0);
            let base = if checker == 0 { 0.85 } else { 0.38 };
            let intensity = (base * ring_glow * depth_fog).clamp(0.0, 1.0);
            let locked = !cell.is_fill && cell.dist_center <= reveal_r;
            paint_math_cell(ctx, cell, &self.palette, intensity, locked, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
