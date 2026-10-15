// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 16-bit SNES arcade post-processing (bevel, specular gleam, drop shadow, parallax starfield)
//! and scene dissolve transitions.

use super::{banner_bevel_color, find_solid_banner_bounds, paint_starfield_backdrop};
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

    paint_starfield_backdrop(grid, cols, rows, frame_tick, backdrop_seed, bg);
}

/// Blends the outgoing frame and incoming frame over progress `t` in `[0.0, 1.0]`.
pub fn paint_transition(
    from: &[TerminalCell],
    to: &[TerminalCell],
    out: &mut [TerminalCell],
    t: f32,
    cols: usize,
    rows: usize,
) {
    let t = t.clamp(0.0, 1.0);
    let total = cols * rows;
    let n = total.min(from.len()).min(to.len()).min(out.len());
    if n == 0 {
        return;
    }

    let inv_t = 1.0 - t;
    let out_factor = (inv_t * 256.0) as u16;
    let in_factor = (t * 256.0) as u16;
    let t_byte = (t * 255.0) as u8;

    for y in 0..rows {
        let row_offset = y * cols;
        for x in 0..cols {
            let idx = row_offset + x;
            if idx >= n {
                break;
            }
            let from_cell = from[idx];
            let to_cell = to[idx];

            let thresh = ((x.wrapping_mul(137) ^ y.wrapping_mul(149) ^ ((x + y).wrapping_mul(31)))
                & 0xFF) as u8;
            let pick_to = t >= 1.0 || (t > 0.0 && t_byte > thresh);
            let factor = if pick_to {
                200 + ((in_factor * 56) >> 8)
            } else {
                200 + ((out_factor * 56) >> 8)
            };
            let src = if pick_to { to_cell } else { from_cell };
            let on_wavefront = t > 0.0 && t < 1.0 && t_byte.abs_diff(thresh) <= 24;
            out[idx] = if src.ch == ' ' {
                TerminalCell::default()
            } else {
                TerminalCell {
                    ch: src.ch,
                    fg: if on_wavefront {
                        (185, 248, 255)
                    } else {
                        scale_rgb(src.fg, factor)
                    },
                    bg: src.bg,
                    bold: on_wavefront || src.bold,
                }
            };
        }
    }

    if out.len() > n {
        out[n..].fill(TerminalCell::default());
    }
}

#[inline(always)]
fn scale_rgb(c: (u8, u8, u8), factor_256: u16) -> (u8, u8, u8) {
    (
        ((c.0 as u16 * factor_256) >> 8) as u8,
        ((c.1 as u16 * factor_256) >> 8) as u8,
        ((c.2 as u16 * factor_256) >> 8) as u8,
    )
}
