// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! High-resolution 4×8 sub-cell sprite overlay publisher and Scale2x compositor.

use crate::cell::TerminalCell;
use crate::wayland::blit::pack_argb2101010;
use std::sync::Mutex;

pub const SUBCELL_X: usize = 4;
pub const SUBCELL_Y: usize = 8;

/// Background RGB of the preview HUD pill (`glyph_font::draw_preview_hud`), excluded from overlay.
pub const HUD_PILL_BG: (u8, u8, u8) = (14, 22, 42);

struct SubcellOverlay {
    cols: usize,
    rows: usize,
    generation: u64,
    palette: [u32; 40],
    pixels: Vec<u8>,
}

static SUBCELL_OVERLAY: Mutex<SubcellOverlay> = Mutex::new(SubcellOverlay {
    cols: 0,
    rows: 0,
    generation: 0,
    palette: [0; 40],
    pixels: Vec::new(),
});

pub fn clear_subcell_overlay() {
    if let Ok(mut ov) = SUBCELL_OVERLAY.lock() {
        (ov.cols, ov.rows, ov.generation) = (0, 0, ov.generation.wrapping_add(1));
    }
}

pub fn release_subcell_overlay() {
    if let Ok(mut ov) = SUBCELL_OVERLAY.lock() {
        (ov.cols, ov.rows, ov.generation) = (0, 0, ov.generation.wrapping_add(1));
        ov.palette.fill(0);
        ov.pixels.clear();
        ov.pixels.shrink_to_fit();
    }
}

pub fn subcell_overlay_generation() -> u64 {
    SUBCELL_OVERLAY.lock().map_or(0, |ov| ov.generation)
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
    if let Ok(mut ov) = SUBCELL_OVERLAY.lock() {
        publish_locked(&mut ov, cols, rows, palette_rgb, px_buf);
    }
}

/// Publishes into an already-locked overlay instance.
fn publish_locked(
    ov: &mut SubcellOverlay,
    cols: usize,
    rows: usize,
    palette_rgb: &[(u8, u8, u8)],
    px_buf: &[u8],
) -> bool {
    let needed = cols
        .saturating_mul(SUBCELL_X)
        .saturating_mul(rows)
        .saturating_mul(SUBCELL_Y);
    if needed == 0 || px_buf.len() < needed {
        return false;
    }
    (ov.cols, ov.rows, ov.generation) = (cols, rows, ov.generation.wrapping_add(1));
    ov.palette.fill(0);
    for (i, &(r, g, b)) in palette_rgb.iter().take(40).enumerate() {
        ov.palette[i] = 0xFF00_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
    }
    if ov.pixels.len() != needed {
        ov.pixels.resize(needed, 0);
    }
    ov.pixels.copy_from_slice(&px_buf[..needed]);
    true
}

pub fn composite_subcell_overlay(
    cells: &[TerminalCell],
    dims: (usize, usize),
    buffer: &mut [u32],
    px_dims: (usize, usize),
    drawn_dims: (usize, usize),
) {
    composite_subcell_overlay_with_format(cells, dims, buffer, px_dims, drawn_dims, false);
}

pub fn composite_subcell_overlay_with_format(
    cells: &[TerminalCell],
    (cols, rows): (usize, usize),
    buffer: &mut [u32],
    (pixel_w, pixel_h): (usize, usize),
    (drawn_w, drawn_h): (usize, usize),
    is_10bit_hdr: bool,
) {
    let Ok(ov) = SUBCELL_OVERLAY.lock() else {
        return;
    };
    composite_locked(
        &ov,
        cells,
        (cols, rows),
        buffer,
        (pixel_w, pixel_h),
        (drawn_w, drawn_h),
        is_10bit_hdr,
    );
}

/// Composites an already-locked overlay instance.
fn composite_locked(
    ov: &SubcellOverlay,
    cells: &[TerminalCell],
    (cols, rows): (usize, usize),
    buffer: &mut [u32],
    (pixel_w, pixel_h): (usize, usize),
    (drawn_w, drawn_h): (usize, usize),
    is_10bit_hdr: bool,
) {
    let (sub_w, sub_h) = (
        cols.saturating_mul(SUBCELL_X),
        rows.saturating_mul(SUBCELL_Y),
    );
    let needed = sub_w.saturating_mul(sub_h);
    if ov.cols != cols || ov.rows != rows || needed == 0 || ov.pixels.len() < needed {
        return;
    }
    let mut hdr_pal = [0u32; 40];
    let pal = if is_10bit_hdr {
        for (dst, &v) in hdr_pal.iter_mut().zip(ov.palette.iter()) {
            let (r, g, b) = (
                ((v >> 16) & 0xFF) as u8,
                ((v >> 8) & 0xFF) as u8,
                (v & 0xFF) as u8,
            );
            *dst = pack_argb2101010((r, g, b));
        }
        &hdr_pal
    } else {
        &ov.palette
    };
    composite_overlay_pixels(
        cells,
        (cols, sub_w, sub_h),
        &ov.pixels,
        pal,
        buffer,
        (pixel_w, pixel_h),
        (drawn_w, drawn_h),
    );
}

/// Inputs for the atomic publish-and-composite test helper.
#[cfg(test)]
pub(crate) struct OverlayTestSpec<'a> {
    pub cols: usize,
    pub rows: usize,
    pub palette_rgb: &'a [(u8, u8, u8)],
    pub px_buf: &'a [u8],
    pub cells: &'a [TerminalCell],
    pub px_dims: (usize, usize),
    pub drawn_dims: (usize, usize),
    pub is_10bit_hdr: bool,
}

/// Publishes and composites in one critical section.
///
/// The overlay is process-global, so a scene-driven publish/clear from another
/// thread could otherwise land between the two halves of this operation.
#[cfg(test)]
pub(crate) fn publish_and_composite_for_test(spec: &OverlayTestSpec<'_>, buffer: &mut [u32]) {
    let Ok(mut ov) = SUBCELL_OVERLAY.lock() else {
        return;
    };
    publish_locked(&mut ov, spec.cols, spec.rows, spec.palette_rgb, spec.px_buf);
    composite_locked(
        &ov,
        spec.cells,
        (spec.cols, spec.rows),
        buffer,
        spec.px_dims,
        spec.drawn_dims,
        spec.is_10bit_hdr,
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
        (pal[0], pal[1]) = (pack_argb((16, 240, 96)), pack_argb((32, 255, 128)));

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
        assert_eq!(
            (buf[0], buf[pw * 2 + 2], buf[5]),
            (0xFF10_F060, 0xFF10_F060, 0xFF00_0000)
        );

        let mut hdr = vec![0xC000_0000u32; pw * ph];
        publish_and_composite_for_test(
            &OverlayTestSpec {
                cols,
                rows,
                palette_rgb: &[(16, 240, 96), (32, 255, 128)],
                px_buf: &px,
                cells: &cells,
                px_dims: (pw, ph),
                drawn_dims: (pw, ph),
                is_10bit_hdr: true,
            },
            &mut hdr,
        );
        assert_eq!(hdr[0], pack_argb2101010((16, 240, 96)));
        clear_subcell_overlay();
        assert!(!has_subcell_overlay(cols, rows));
        release_subcell_overlay();
        assert!(!has_subcell_overlay(cols, rows));
    }
}
