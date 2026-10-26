// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Scale2x pixel compositor for the 4x8 sub-cell sprite overlay.

use crate::cell::TerminalCell;

use super::{HUD_PILL_BG, SUBCELL_X, SUBCELL_Y};

#[inline]
pub(in crate::wayland) fn composite_overlay_pixels(
    cells: &[TerminalCell],
    (cols, sub_w, sub_h): (usize, usize, usize),
    pixels: &[u8],
    palette: &[u32; 40],
    buffer: &mut [u32],
    (pixel_w, pixel_h): (usize, usize),
    (drawn_w, drawn_h): (usize, usize),
) {
    let get_px = |x: usize, y: usize| pixels.get(y * sub_w + x).copied().unwrap_or(0);
    for sy in 0..sub_h {
        let (cy, py0) = (sy / SUBCELL_Y, (sy * drawn_h) / sub_h);
        let py1 = (((sy + 1) * drawn_h) / sub_h).max(py0 + 1).min(pixel_h);
        for sx in 0..sub_w {
            if cells
                .get(cy * cols + sx / SUBCELL_X)
                .is_some_and(|c| c.bg == HUD_PILL_BG)
            {
                continue;
            }
            let p = get_px(sx, sy);
            let a = if sy > 0 { get_px(sx, sy - 1) } else { 0 };
            let b = if sx + 1 < sub_w {
                get_px(sx + 1, sy)
            } else {
                0
            };
            let c = if sx > 0 { get_px(sx - 1, sy) } else { 0 };
            let d = if sy + 1 < sub_h {
                get_px(sx, sy + 1)
            } else {
                0
            };
            let q = |u: u8, v: u8, ou: u8, ov: u8| {
                if u != 0 && v != 0 && u != ou && v != ov && u.abs_diff(v) <= 1 {
                    u.min(v)
                } else {
                    p
                }
            };
            let e = [q(c, a, d, b), q(a, b, c, d), q(d, c, b, a), q(b, d, a, c)];
            if (e[0] | e[1] | e[2] | e[3]) == 0 {
                continue;
            }
            let px0 = (sx * drawn_w) / sub_w;
            let px1 = (((sx + 1) * drawn_w) / sub_w).max(px0 + 1).min(pixel_w);
            let (pxm, pym) = ((px0 + px1).div_ceil(2), (py0 + py1).div_ceil(2));
            for py in py0..py1 {
                let (row_off, qy) = (py * pixel_w, usize::from(py >= pym) * 2);
                for px in px0..px1 {
                    let idx = e[qy + usize::from(px >= pxm)];
                    if idx != 0 && row_off + px < buffer.len() {
                        buffer[row_off + px] = palette[(idx as usize - 1).min(39)];
                    }
                }
            }
        }
    }
}
