// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal cell buffer drawing and dissolve transitions.

use crate::cell::TerminalCell;

/// Smoothly blends the outgoing frame and incoming frame over progress `t` in `[0.0, 1.0]`.
///
/// Implements an energy-preserving dissolve transition:
/// - Eliminates all hard cuts, screen blanking, pops, and flashes.
/// - Outgoing settled text gracefully fades in luminance and dissolves.
/// - Incoming effect animation ramps in brightness and enters cleanly.
/// - Zero heap allocation, bounds-checked.
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

            // Pseudo-random pseudo-dither threshold based on coordinates
            let thresh = ((x.wrapping_mul(137) ^ y.wrapping_mul(149) ^ ((x + y).wrapping_mul(31)))
                & 0xFF) as u8;
            let pick_to = t >= 1.0 || (t > 0.0 && t_byte > thresh);
            let factor = if pick_to {
                180 + ((in_factor * 76) >> 8)
            } else {
                180 + ((out_factor * 76) >> 8)
            };
            let src = if pick_to { to_cell } else { from_cell };
            out[idx] = if src.ch == ' ' {
                TerminalCell::default()
            } else {
                TerminalCell {
                    ch: src.ch,
                    fg: scale_rgb(src.fg, factor),
                    bg: src.bg,
                    bold: src.bold,
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

/// Blits banner lines onto a target grid centered horizontally and vertically.
pub fn place_banner_centered(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    lines: &[String],
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) -> (usize, usize, usize, usize) {
    if cols == 0 || rows == 0 || lines.is_empty() {
        return (0, 0, 0, 0);
    }
    let banner_h = lines.len();
    let banner_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

    let start_x = cols.saturating_sub(banner_w) / 2;
    let start_y = rows.saturating_sub(banner_h) / 2;

    for (row_idx, line) in lines.iter().enumerate() {
        let y = start_y + row_idx;
        if y >= rows {
            break;
        }
        for (col_idx, ch) in line.chars().enumerate() {
            let x = start_x + col_idx;
            if x >= cols {
                break;
            }
            if ch != ' ' {
                let idx = y * cols + x;
                if idx < grid.len() {
                    grid[idx] = TerminalCell {
                        ch,
                        fg,
                        bg,
                        bold: true,
                    };
                }
            }
        }
    }

    (start_x, start_y, banner_w, banner_h)
}
