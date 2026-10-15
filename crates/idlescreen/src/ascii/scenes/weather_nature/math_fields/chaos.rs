// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 3D Lorenz strange attractor ODE phase-space trajectory effect settling into the banner.

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
const TRAIL_LEN: usize = 48;
const PALETTE_STOPS: [&str; 5] = ["#24083C", "#7B1FA2", "#E53935", "#FFB300", "#FFF8E1"];
const FINAL_STOPS: [&str; 3] = ["#E53935", "#FFB300", "#FFF8E1"];

pub struct Chaos {
    cells: Vec<CanvasCell>,
    palette: Vec<Color>,
    state: (f64, f64, f64),
    trail: Vec<(f64, f64)>,
    frame: u32,
}

impl Chaos {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            palette: Vec::new(),
            state: (0.1, 0.0, 0.0),
            trail: Vec::with_capacity(TRAIL_LEN),
            frame: 0,
        }
    }

    fn step_lorenz(&mut self) {
        let sigma = 10.0;
        let rho = 28.0;
        let beta = 8.0 / 3.0;
        let dt = 0.0085;
        for _ in 0..4 {
            let (x, y, z) = self.state;
            let dx = sigma * (y - x);
            let dy = x * (rho - z) - y;
            let dz = x * y - beta * z;
            self.state = (x + dx * dt, y + dy * dt, z + dz * dt);
            let px = (self.state.0 / 22.0).clamp(-1.4, 1.4);
            let py = ((self.state.2 - 25.0) / 22.0).clamp(-1.0, 1.0);
            if self.trail.len() >= TRAIL_LEN {
                self.trail.remove(0);
            }
            self.trail.push((px, py));
        }
    }
}

impl Default for Chaos {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHooks for Chaos {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Chaos {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.cells = build_canvas_cells(ctx, &FINAL_STOPS)?;
        self.palette = build_palette(&PALETTE_STOPS, 12)?;
        self.state = (0.1, 0.0, 0.0);
        self.trail.clear();
        for _ in 0..12 {
            self.step_lorenz();
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

        self.step_lorenz();
        let progress = self.frame as f64 / TOTAL_FRAMES as f64;
        let fill_fade = if progress > 0.72 {
            ((1.0 - progress) / 0.28).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let reveal_radius = progress * 1.9;
        let n_trail = self.trail.len().max(1) as f64;

        for cell in &self.cells {
            let mut best = 0.0f64;
            for (idx, &(tx, ty)) in self.trail.iter().enumerate() {
                let d2 = (cell.dx - tx).powi(2) + (cell.dy - ty).powi(2);
                if d2 < 0.18 {
                    let age = (idx + 1) as f64 / n_trail;
                    let glow = (-d2 * 22.0).exp() * (0.35 + 0.65 * age);
                    if glow > best {
                        best = glow;
                    }
                }
            }
            let lobe_l = ((cell.dx + 0.45).powi(2) + cell.dy.powi(2)).sqrt();
            let lobe_r = ((cell.dx - 0.45).powi(2) + cell.dy.powi(2)).sqrt();
            let swirl = 0.32 * (-(lobe_l.min(lobe_r) * 3.2)).exp();
            let intensity = (best + swirl).clamp(0.0, 1.0);
            let locked = !cell.is_fill && cell.dist_center <= reveal_radius;
            paint_math_cell(ctx, cell, &self.palette, intensity, locked, fill_fade);
        }

        Some(emit_frame(ctx))
    }
}
