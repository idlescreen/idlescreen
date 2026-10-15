// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Bouncing Qix / Mystify neon polygon laser ribbons with phosphor persistence trails.

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
const PALETTE_STOPS: [&str; 5] = ["#190033", "#7209B7", "#F72585", "#4CC9F0", "#FFF8E7"];
const FINAL_STOPS: [&str; 3] = ["#F72585", "#4CC9F0", "#FFF8E7"];

pub struct Mystify {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    verts: [(f64, f64, f64, f64); 4],
    history: Vec<[(f64, f64); 4]>,
    frame: u32,
}

impl Mystify {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            verts: [
                (-0.85, -0.55, 0.062, 0.048),
                (0.75, -0.65, -0.054, 0.058),
                (0.82, 0.62, -0.060, -0.045),
                (-0.72, 0.68, 0.051, -0.056),
            ],
            history: Vec::with_capacity(4),
            frame: 0,
        }
    }

    fn step_polygon(&mut self) {
        for v in &mut self.verts {
            v.0 += v.2;
            v.1 += v.3;
            if v.0 <= -1.25 || v.0 >= 1.25 {
                v.2 = -v.2;
                v.0 = v.0.clamp(-1.25, 1.25);
            }
            if v.1 <= -0.92 || v.1 >= 0.92 {
                v.3 = -v.3;
                v.1 = v.1.clamp(-0.92, 0.92);
            }
        }
        let snap = [
            (self.verts[0].0, self.verts[0].1),
            (self.verts[1].0, self.verts[1].1),
            (self.verts[2].0, self.verts[2].1),
            (self.verts[3].0, self.verts[3].1),
        ];
        if self.history.len() >= 4 {
            self.history.remove(0);
        }
        self.history.push(snap);
    }
}

fn seg_dist_sq(px: f64, py: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let vx = b.0 - a.0;
    let vy = b.1 - a.1;
    let len_sq = vx * vx + vy * vy;
    let t = if len_sq > 1e-6 {
        (((px - a.0) * vx + (py - a.1) * vy) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let cx = a.0 + t * vx;
    let cy = a.1 + t * vy;
    (px - cx).powi(2) + (py - cy).powi(2)
}

impl Default for Mystify {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Mystify {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Mystify {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.cells = build_canvas_cells(ctx, &FINAL_STOPS)?;
        self.palette = build_palette(&PALETTE_STOPS, 12)?;
        self.verts = [
            (-0.85, -0.55, 0.062, 0.048),
            (0.75, -0.65, -0.054, 0.058),
            (0.82, 0.62, -0.060, -0.045),
            (-0.72, 0.68, 0.051, -0.056),
        ];
        self.history.clear();
        for _ in 0..3 {
            self.step_polygon();
        }
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

        self.step_polygon();
        let progress = self.frame as f64 / TOTAL_FRAMES as f64;
        let fill_fade = if progress > 0.72 {
            ((1.0 - progress) / 0.28).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let reveal_r = progress * 1.85;
        let n_hist = self.history.len().max(1) as f64;

        for cell in &self.cells {
            let mut best = 0.0f64;
            for (idx, poly) in self.history.iter().enumerate() {
                let age = (idx + 1) as f64 / n_hist;
                for edge in 0..4 {
                    let d2 = seg_dist_sq(cell.dx, cell.dy, poly[edge], poly[(edge + 1) % 4]);
                    if d2 < 0.045 {
                        let glow = (-d2 * 65.0).exp() * age;
                        if glow > best {
                            best = glow;
                        }
                    }
                }
            }
            let locked = !cell.is_fill && cell.dist_center <= reveal_r;
            paint_math_cell(ctx, cell, &self.palette, best, locked, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
