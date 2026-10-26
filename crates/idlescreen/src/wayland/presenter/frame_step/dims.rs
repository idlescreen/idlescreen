// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Buffer dimension resolution for the compositor surface.

use crate::raster::blit::{GLYPH_HEIGHT, GLYPH_WIDTH};
use crate::wayland::fractional;

pub fn viewport_buffer_dimensions(
    width: u32,
    height: u32,
    cols: usize,
    rows: usize,
    scale_120ths: u32,
) -> (u32, u32) {
    if scale_120ths > 120 {
        // The compositor reported a scale above 100%: size the buffer with
        // the standard fractional-scale rounding so the viewport scales the
        // glyph grid without blurring it.
        let cell_w = u32::try_from(cols * GLYPH_WIDTH).unwrap_or(width);
        let cell_h = u32::try_from(rows * GLYPH_HEIGHT).unwrap_or(height);
        return (
            fractional::calculate_buffer_dimension(cell_w, scale_120ths),
            fractional::calculate_buffer_dimension(cell_h, scale_120ths),
        );
    }
    // No scale reported yet: fall back to the 4K heuristic.
    let scale = if width >= 3840 && height >= 2160 {
        2
    } else {
        1
    };
    let bw = u32::try_from(cols * GLYPH_WIDTH * scale).unwrap_or(width);
    let bh = u32::try_from(rows * GLYPH_HEIGHT * scale).unwrap_or(height);
    (bw, bh)
}

#[inline]
pub(super) fn resolve_surface_dims(cw: u32, ch: u32, w: u32, h: u32) -> (u32, u32) {
    let rw = [cw, w, 1920].into_iter().find(|&v| v > 0).unwrap_or(1920);
    let rh = [ch, h, 1080].into_iter().find(|&v| v > 0).unwrap_or(1080);
    (rw, rh)
}
