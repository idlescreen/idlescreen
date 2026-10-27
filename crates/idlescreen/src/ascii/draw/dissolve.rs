// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! The scene dissolve: one scene handing over to the next.
//!
//! The wipe is random: the direction it travels and the shape of its edge are
//! drawn from the seed, so no two transitions look alike. It is not random
//! per cell, though — the front is a single coherent line, roughened by a
//! smooth noise field. That is the difference between a wipe and static:
//! per-cell randomness shreds a block glyph into confetti, while a coherent
//! front cuts it cleanly.
//!
//! The colour cross-fades continuously across the whole dissolve, so the
//! transition still reads when both sides carry the same colour — the case
//! where a hard per-cell swap has nothing but the swap to show.

use crate::cell::TerminalCell;

/// The lattice spacing, in cells, of the random field that roughens the front.
///
/// Small enough that the front breaks into an organic edge, large enough that
/// neighbouring cells stay close together. A one-cell lattice would be white
/// noise again — and white noise is what made the old wipe read as static.
const NOISE_CELL: usize = 6;

/// The colour of the rim that rides the wavefront.
const WAVEFRONT_RGB: (u8, u8, u8) = (185, 248, 255);

/// How far the front's thresholds are pulled away from the ends of the
/// dissolve, so the first and last cells are not the very first to go.
const WIPE_MARGIN: f32 = 0.03;

/// How far the random edge may push the front, in dissolve progress.
const EDGE_JITTER: f32 = 0.16;

/// How wide the wavefront rim is, in dissolve progress.
const WAVEFRONT_WIDTH: f32 = 0.07;

/// Blends the outgoing frame and incoming frame over progress `t` in `[0.0, 1.0]`.
pub fn paint_transition(
    from: &[TerminalCell],
    to: &[TerminalCell],
    out: &mut [TerminalCell],
    t: f32,
    cols: usize,
    rows: usize,
    seed: u64,
) {
    let t = t.clamp(0.0, 1.0);
    let total = cols * rows;
    let n = total.min(from.len()).min(to.len()).min(out.len());
    if n == 0 {
        return;
    }

    let (ca, sa) = sweep_direction(seed);

    for y in 0..rows {
        let row_offset = y * cols;
        for x in 0..cols {
            let idx = row_offset + x;
            if idx >= n {
                break;
            }
            let from_cell = from[idx];
            let to_cell = to[idx];

            let threshold = wipe_threshold(x, y, cols, rows, seed, ca, sa);
            let pick_to = t >= threshold;

            // A blank cell carries no foreground of its own, so it fades
            // against its own background rather than against black.
            let from_fg = if from_cell.ch == ' ' {
                from_cell.bg
            } else {
                from_cell.fg
            };
            let to_fg = if to_cell.ch == ' ' {
                to_cell.bg
            } else {
                to_cell.fg
            };
            let blended = lerp_rgb(from_fg, to_fg, t);

            let src = if pick_to { to_cell } else { from_cell };
            let on_wavefront = t > 0.0 && t < 1.0 && (t - threshold).abs() <= WAVEFRONT_WIDTH;
            out[idx] = if src.ch == ' ' {
                TerminalCell {
                    bg: src.bg,
                    ..TerminalCell::default()
                }
            } else {
                TerminalCell {
                    ch: src.ch,
                    fg: if on_wavefront { WAVEFRONT_RGB } else { blended },
                    bg: src.bg,
                    bold: on_wavefront || src.bold,
                }
            };
        }
    }

    if out.len() > n {
        let fill_bg = to.first().map_or((0, 0, 0), |c| c.bg);
        out[n..].fill(TerminalCell {
            bg: fill_bg,
            ..TerminalCell::default()
        });
    }
}

/// The dissolve progress at which the cell at `(x, y)` changes over.
///
/// The front travels along a line at the sweep's angle, roughened by a
/// smooth random field so the edge is organic rather than ruler-straight.
fn wipe_threshold(
    x: usize,
    y: usize,
    cols: usize,
    rows: usize,
    seed: u64,
    ca: f32,
    sa: f32,
) -> f32 {
    let fx = (x as f32 + 0.5) / cols.max(1) as f32 - 0.5;
    let fy = (y as f32 + 0.5) / rows.max(1) as f32 - 0.5;
    let extent = 0.5 * (ca.abs() + sa.abs()).max(1e-6);
    let along = ((fx * ca + fy * sa) / extent + 1.0) * 0.5;
    let jitter = (smooth_noise(x, y, seed) - 0.5) * EDGE_JITTER;
    let raw = along + jitter;
    let lo = WIPE_MARGIN;
    let hi = 1.0 - WIPE_MARGIN;
    (lo + raw.clamp(0.0, 1.0) * (hi - lo)).clamp(lo, hi)
}

/// The direction the wipe travels, as `(cos, sin)`, drawn from the seed.
fn sweep_direction(seed: u64) -> (f32, f32) {
    let angle = lattice(seed, 0, 0) * std::f32::consts::TAU;
    (angle.cos(), angle.sin())
}

/// A smooth random field over the cell grid, in `[0.0, 1.0]`.
///
/// Value noise on a coarse lattice: random values every [`NOISE_CELL`] cells,
/// smoothly interpolated between them. Adjacent cells land close together,
/// which is what keeps the wipe's edge from breaking into static.
#[inline]
fn smooth_noise(x: usize, y: usize, seed: u64) -> f32 {
    let gx = (x / NOISE_CELL) as u64;
    let gy = (y / NOISE_CELL) as u64;
    let fx = (x % NOISE_CELL) as f32 / NOISE_CELL as f32;
    let fy = (y % NOISE_CELL) as f32 / NOISE_CELL as f32;
    // Smoothstep the cell-local coordinates so the lattice seams do not show.
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    let v00 = lattice(seed, gx, gy);
    let v10 = lattice(seed, gx + 1, gy);
    let v01 = lattice(seed, gx, gy + 1);
    let v11 = lattice(seed, gx + 1, gy + 1);

    let top = v00 + (v10 - v00) * ux;
    let bottom = v01 + (v11 - v01) * ux;
    top + (bottom - top) * uy
}

/// One random lattice value in `[0.0, 1.0)`, derived from `seed` and the
/// lattice coordinates.
#[inline]
fn lattice(seed: u64, gx: u64, gy: u64) -> f32 {
    let mut z = seed
        .wrapping_add(gx.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(gy.wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    ((z >> 40) as f32) / 16_777_216.0
}

/// Linear interpolation between two colours.
#[inline]
fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let mix = |x: u8, y: u8| {
        (f32::from(x) + (f32::from(y) - f32::from(x)) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    (mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}
