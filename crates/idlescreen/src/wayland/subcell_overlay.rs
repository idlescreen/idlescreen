// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! High-resolution 4×8 sub-cell sprite overlay publisher and Scale2x compositor.

use crate::cell::TerminalCell;
use std::sync::Mutex;

pub const SUBCELL_X: usize = 4;
pub const SUBCELL_Y: usize = 8;

/// Background RGB of the preview HUD pill (`glyph_font::draw_preview_hud`), excluded from overlay.
pub const HUD_PILL_BG: (u8, u8, u8) = (14, 22, 42);

struct SubcellOverlay {
    cols: usize,
    rows: usize,
    palette: [u32; 40],
    pixels: Vec<u8>,
}

static SUBCELL_OVERLAY: Mutex<SubcellOverlay> = Mutex::new(SubcellOverlay {
    cols: 0,
    rows: 0,
    palette: [0; 40],
    pixels: Vec::new(),
});

pub fn clear_subcell_overlay() {
    if let Ok(mut ov) = SUBCELL_OVERLAY.lock() {
        (ov.cols, ov.rows) = (0, 0);
    }
}

pub fn has_subcell_overlay(cols: usize, rows: usize) -> bool {
    let needed = cols
        .saturating_mul(SUBCELL_X)
        .saturating_mul(rows)
        .saturating_mul(SUBCELL_Y);
    needed > 0
        && SUBCELL_OVERLAY
            .lock()
            .is_ok_and(|ov| ov.cols == cols && ov.rows == rows && ov.pixels.len() >= needed)
}

pub fn publish_subcell_overlay(
    cols: usize,
    rows: usize,
    palette_rgb: &[(u8, u8, u8)],
    px_buf: &[u8],
) {
    let needed = cols
        .saturating_mul(SUBCELL_X)
        .saturating_mul(rows)
        .saturating_mul(SUBCELL_Y);
    if needed == 0 || px_buf.len() < needed {
        return;
    }
    if let Ok(mut ov) = SUBCELL_OVERLAY.lock() {
        (ov.cols, ov.rows) = (cols, rows);
        ov.palette.fill(0);
        for (i, &(r, g, b)) in palette_rgb.iter().take(40).enumerate() {
            ov.palette[i] = 0xFF00_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
        }
        if ov.pixels.len() != needed {
            ov.pixels.resize(needed, 0);
        }
        ov.pixels.copy_from_slice(&px_buf[..needed]);
    }
}

pub fn composite_subcell_overlay(
    cells: &[TerminalCell],
    (cols, rows): (usize, usize),
    buffer: &mut [u32],
    (pixel_w, pixel_h): (usize, usize),
    (drawn_w, drawn_h): (usize, usize),
) {
    let (sub_w, sub_h) = (
        cols.saturating_mul(SUBCELL_X),
        rows.saturating_mul(SUBCELL_Y),
    );
    let needed = sub_w.saturating_mul(sub_h);
    let Ok(ov) = SUBCELL_OVERLAY.lock() else {
        return;
    };
    if ov.cols != cols || ov.rows != rows || needed == 0 || ov.pixels.len() < needed {
        return;
    }
    composite_overlay_pixels(
        cells,
        (cols, sub_w, sub_h),
        &ov.pixels,
        &ov.palette,
        buffer,
        (pixel_w, pixel_h),
        (drawn_w, drawn_h),
    );
}

#[inline]
pub(super) fn composite_overlay_pixels(
    cells: &[TerminalCell],
    (cols, sub_w, sub_h): (usize, usize, usize),
    pixels: &[u8],
    palette: &[u32; 40],
    buffer: &mut [u32],
    (pixel_w, pixel_h): (usize, usize),
    (drawn_w, drawn_h): (usize, usize),
) {
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
            let p = pixels.get(sy * sub_w + sx).copied().unwrap_or(0);
            let a = if sy > 0 {
                pixels.get((sy - 1) * sub_w + sx).copied().unwrap_or(0)
            } else {
                0
            };
            let b = if sx + 1 < sub_w {
                pixels.get(sy * sub_w + sx + 1).copied().unwrap_or(0)
            } else {
                0
            };
            let c = if sx > 0 {
                pixels.get(sy * sub_w + sx - 1).copied().unwrap_or(0)
            } else {
                0
            };
            let d = if sy + 1 < sub_h {
                pixels.get((sy + 1) * sub_w + sx).copied().unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::blit::{GLYPH_HEIGHT, GLYPH_WIDTH, pack_argb};

    #[test]
    fn test_subcell_overlay_blitting() {
        let (cols, rows) = (4, 2);
        let (pw, ph, sw, sh) = (
            cols * GLYPH_WIDTH,
            rows * GLYPH_HEIGHT,
            cols * SUBCELL_X,
            rows * SUBCELL_Y,
        );
        let mut px = vec![0u8; sw * sh];
        (px[0], px[1], px[sw], px[sw + 2]) = (1, 1, 2, 2);
        let mut pal = [0u32; 40];
        pal[0] = pack_argb((16, 240, 96));
        pal[1] = pack_argb((32, 255, 128));

        let mut cells = vec![TerminalCell::default(); cols * rows];
        (cells[0].ch, cells[0].bold) = ('▀', false);
        let mut buf = vec![0xFF00_0000u32; pw * ph];
        composite_overlay_pixels(
            &cells,
            (cols, sw, sh),
            &px,
            &pal,
            &mut buf,
            (pw, ph),
            (pw, ph),
        );
        assert_eq!(buf[0], 0xFF10_F060);
        assert_eq!(buf[pw * 2 + 2], 0xFF10_F060);
        assert_eq!(buf[5], 0xFF00_0000);

        publish_subcell_overlay(cols, rows, &[(16, 240, 96), (32, 255, 128)], &px);
        clear_subcell_overlay();
        assert!(!has_subcell_overlay(cols, rows));
    }
}
