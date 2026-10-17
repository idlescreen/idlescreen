// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! SNES Mega Man X2 / X3 Zero Effect state machine, sub-cell Wayland overlay
//! publisher, and diagonal Z-Saber slice half-block terminal canvas emitter.

use super::zero_choreo::{compose_frame_pixels, destroy_front_for_row};
use super::zero_data::{atlas, build_zero_colors, sample_subcell_halfblock, zero_palette_rgb};
use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use crate::ascii::scenes::weather_nature::field_ops::{CanvasCell, build_canvas_cells};
use crate::wayland::blit::{clear_subcell_overlay, publish_subcell_overlay};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::utils::graphics::{Color, ColorPair};

const TOTAL_FRAMES: u32 = 120;
const FINAL_STOPS: [&str; 3] = ["#14F4FF", "#10C8FF", "#0868D8"];

pub struct Zero {
    cells: Vec<CanvasCell>,
    pal_colors: Vec<Color>,
    transparent_bg: Color,
    flash_emerald: ColorPair,
    flash_magenta: ColorPair,
    flash_gold: ColorPair,
    px_buf: Vec<u8>,
    cols: usize,
    rows: usize,
    text_bounds: (usize, usize, usize, usize),
    frame: u32,
}

impl Default for Zero {
    fn default() -> Self {
        Self::new()
    }
}

impl Zero {
    pub fn new() -> Self {
        let transparent_bg = hex_color("#000001");
        Self {
            cells: Vec::new(),
            pal_colors: Vec::new(),
            flash_emerald: ColorPair::new(Some(hex_color("#40FF80")), Some(transparent_bg)),
            flash_magenta: ColorPair::new(Some(hex_color("#FF38A8")), Some(transparent_bg)),
            flash_gold: ColorPair::new(Some(hex_color("#FFF078")), Some(transparent_bg)),
            transparent_bg,
            px_buf: Vec::new(),
            cols: 80,
            rows: 24,
            text_bounds: (15, 65, 8, 16),
            frame: 0,
        }
    }
}

impl EffectHooks for Zero {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}

impl Effect for Zero {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        self.cells = build_canvas_cells(ctx, &FINAL_STOPS)?;
        self.pal_colors = build_zero_colors();
        self.cols = (ctx.terminal.canvas.right - ctx.terminal.canvas.left + 1).max(1) as usize;
        self.rows = (ctx.terminal.canvas.top - ctx.terminal.canvas.bottom + 1).max(1) as usize;
        let left = ctx.terminal.canvas.left;
        let top = ctx.terminal.canvas.top;
        let (mut min_x, mut max_x) = (self.cols, 0usize);
        let (mut min_y, mut max_y) = (self.rows, 0usize);
        for cell in &self.cells {
            if !cell.is_fill {
                let x = (cell.col - left).max(0) as usize;
                let y = (top - cell.row).max(0) as usize;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
        if min_x > max_x {
            min_x = (ctx.terminal.canvas.text_left - left).max(0) as usize;
            max_x = (ctx.terminal.canvas.text_right - left).max(0) as usize;
        }
        if min_y > max_y {
            (min_y, max_y) = (self.rows / 3, (self.rows * 2) / 3);
        }
        apply_banner_bevel(&mut self.cells, top, min_y, max_y);
        self.text_bounds = (min_x, max_x, min_y, max_y);
        self.px_buf.resize(self.cols * 4 * self.rows * 8, 0);
        self.frame = 0;
        Ok(())
    }

    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String> {
        if self.frame >= TOTAL_FRAMES {
            clear_subcell_overlay();
            return None;
        }
        self.frame += 1;
        let a = atlas();
        compose_frame_pixels(
            &mut self.px_buf,
            (self.cols, self.rows),
            self.text_bounds,
            self.frame,
            a,
        );
        publish_subcell_overlay(self.cols, self.rows, zero_palette_rgb(), &self.px_buf);

        let left = ctx.terminal.canvas.left;
        let top = ctx.terminal.canvas.top;
        let sub_w = self.cols * 4;

        for cell in &self.cells {
            let x = (cell.col - left).max(0) as usize;
            let y = (top - cell.row).max(0) as usize;
            let top_px = sample_subcell_halfblock(&self.px_buf, sub_w, x, y * 2);
            let bot_px = sample_subcell_halfblock(&self.px_buf, sub_w, x, y * 2 + 1);
            let destroy_front =
                destroy_front_for_row((self.cols, self.rows), self.text_bounds, y, self.frame);

            if !cell.is_fill && (x as isize) > destroy_front {
                ctx.terminal.set_character_visibility(cell.id, true);
                let ch = &mut ctx.terminal.arena[cell.id.0 as usize];
                let hit = matches!(self.frame, 30..=38 | 45..=55 | 68..=100);
                let cols = if hit && (x as isize) <= destroy_front + 4 {
                    if self.frame < 67 {
                        &self.flash_gold
                    } else if (x + y + self.frame as usize).is_multiple_of(2) {
                        &self.flash_emerald
                    } else {
                        &self.flash_magenta
                    }
                } else {
                    &cell.settled_colors
                };
                ch.animation.set_appearance(
                    &cell.input_symbol,
                    cell.uses_pre,
                    Some(&cell.input_symbol),
                    Some(*cols),
                );
            } else if top_px != 0 || bot_px != 0 {
                let (sym, fg_idx, bg_opt) = match (top_px, bot_px) {
                    (t, 0) => ("▀", t, Some(self.transparent_bg)),
                    (0, b) => ("▄", b, Some(self.transparent_bg)),
                    (t, b) => (
                        "▀",
                        t,
                        Some(self.pal_colors[(b as usize).saturating_sub(1)]),
                    ),
                };
                let fg = self.pal_colors[(fg_idx as usize).saturating_sub(1)];
                ctx.terminal.set_character_visibility(cell.id, true);
                let ch = &mut ctx.terminal.arena[cell.id.0 as usize];
                ch.animation.set_appearance(
                    &cell.input_symbol,
                    cell.uses_pre,
                    Some(sym),
                    Some(ColorPair::new(Some(fg), bg_opt)),
                );
            } else {
                ctx.terminal.set_character_visibility(cell.id, false);
            }
        }
        Some(emit_frame(ctx))
    }
}

fn apply_banner_bevel(cells: &mut [CanvasCell], top: i64, min_y: usize, max_y: usize) {
    let span = max_y.saturating_sub(min_y) + 1;
    let has_gap = span >= 6
        && (min_y + 1..max_y).any(|ry| {
            !cells
                .iter()
                .any(|c| !c.is_fill && (top - c.row).max(0) as usize == ry)
        });
    let super_rows = if !has_gap {
        0
    } else if span >= 18 {
        6
    } else {
        (span * 3) / 7
    };
    for cell in cells.iter_mut() {
        if cell.is_fill {
            continue;
        }
        let y = (top - cell.row).max(0) as usize;
        let rel_y = y.saturating_sub(min_y);
        let hex = if super_rows > 0 && rel_y < super_rows {
            match (rel_y * 6) / super_rows.max(1) {
                0 | 1 => "#FFF014",
                2 | 3 => "#FFC80A",
                _ => "#E89606",
            }
        } else {
            let f_start = if super_rows > 0 { super_rows + 1 } else { 0 };
            let f_span = span.saturating_sub(f_start).max(1);
            match (rel_y.saturating_sub(f_start) * 8) / f_span {
                0 | 1 => "#14F4FF",
                2 | 3 => "#10C8FF",
                4 | 5 => "#0C98F0",
                _ => "#0868D8",
            }
        };
        cell.settled_colors = ColorPair::new(Some(hex_color(hex)), None);
    }
}
