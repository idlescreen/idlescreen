// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 16-bit SNES arcade post-processing (bevel, specular gleam, drop shadow, parallax starfield).

use super::{
    apply_anamorphic_lens_flare, banner_bevel_color, find_solid_banner_bounds,
    paint_starfield_backdrop,
};
use crate::cell::TerminalCell;

/// Applies 16-bit SNES arcade post-processing, scanning bounds if `settled` is true.
pub fn apply_arcade_fx(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    frame_tick: u64,
    settled: bool,
    bg: (u8, u8, u8),
) {
    let bounds = settled
        .then(|| find_solid_banner_bounds(grid, cols, rows))
        .flatten();
    apply_arcade_fx_with_bounds(grid, cols, rows, frame_tick, 0, bounds, false, bg);
}

/// Applies 16-bit SNES arcade post-processing using pre-cached `settled_bounds` to avoid
/// full-grid bounds rescanning during static dwell (§3.2 #3).
#[allow(clippy::too_many_arguments)]
pub fn apply_arcade_fx_with_bounds(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    frame_tick: u64,
    backdrop_seed: u64,
    settled_bounds: Option<(usize, usize)>,
    is_stacked_superfedora: bool,
    bg: (u8, u8, u8),
) {
    let total = cols.saturating_mul(rows).min(grid.len());
    if total == 0 || cols == 0 || rows == 0 {
        return;
    }

    let tick = frame_tick as usize;
    let gleam_span = (cols + rows * 2 + 28).max(1);
    let gleam_pos = (tick * 3) % gleam_span;

    for y in 0..rows {
        let row_off = y * cols;
        for x in 0..cols {
            let idx = row_off + x;
            if idx >= total {
                break;
            }
            let cell = grid[idx];
            if cell.bg != bg {
                if cell.bg == (0, 0, 1) {
                    grid[idx].bg = bg;
                }
                grid[idx].bold = false;
                continue;
            }
            if cell.ch == '█' || cell.ch == '▀' || cell.ch == '▄' {
                let diag = x + y * 2;
                let dist = diag.abs_diff(gleam_pos);
                let new_fg = if dist == 0 {
                    (245, 255, 255)
                } else if dist == 1 {
                    (255, 225, 110)
                } else if dist == 2 {
                    (130, 235, 255)
                } else if let Some((min_y, max_y)) = settled_bounds {
                    banner_bevel_color(y, min_y, max_y, is_stacked_superfedora)
                } else {
                    cell.fg
                };
                grid[idx].fg = new_fg;
                grid[idx].bold = true;

                if x + 1 < cols && y + 1 < rows {
                    let shadow_idx = (y + 1) * cols + (x + 1);
                    if shadow_idx < total && grid[shadow_idx].ch == ' ' {
                        grid[shadow_idx] = TerminalCell {
                            ch: '░',
                            fg: (36, 66, 138),
                            bg,
                            bold: false,
                        };
                    }
                }
            }
        }
    }

    apply_anamorphic_lens_flare(grid, cols, rows, gleam_pos, bg);
    paint_starfield_backdrop(grid, cols, rows, frame_tick, backdrop_seed, bg);
}
