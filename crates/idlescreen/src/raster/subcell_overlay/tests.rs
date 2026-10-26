// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Tests for the sub-cell sprite overlay publisher and Scale2x compositor.

use super::*;
use crate::raster::blit::{GLYPH_HEIGHT, GLYPH_WIDTH, pack_argb};

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
