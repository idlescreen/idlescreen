// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Direct CPU font blitter into wl_shm ARGB8888 buffers (<0.5ms frame time).

use crate::cell::TerminalCell;
pub use crate::wayland::glyph_font::{GLYPH_HEIGHT, GLYPH_WIDTH, get_glyph_bitmap};
pub use crate::wayland::subcell_overlay::{
    SUBCELL_X, SUBCELL_Y, clear_subcell_overlay, composite_subcell_overlay, has_subcell_overlay,
    publish_subcell_overlay,
};

#[inline]
pub(super) fn pack_argb(c: (u8, u8, u8)) -> u32 {
    0xFF00_0000 | (u32::from(c.0) << 16) | (u32::from(c.1) << 8) | u32::from(c.2)
}

/// Packs 8-bit RGB channels into 10-bit `Argb2101010` (`[31:30] A=3, [29:20] R, [19:10] G, [9:0] B`).
#[inline]
pub fn pack_argb2101010(c: (u8, u8, u8)) -> u32 {
    let expand10 = |v: u8| -> u32 { (u32::from(v) << 2) | (u32::from(v) >> 6) };
    0xC000_0000 | (expand10(c.0) << 20) | (expand10(c.1) << 10) | expand10(c.2)
}

/// Converts an in-place `Argb8888` pixel buffer into 10-bit `Argb2101010` for HDR/deep-color `wl_shm`.
#[inline]
pub fn convert_argb8888_to_2101010_in_place(buffer: &mut [u32]) {
    for px in buffer.iter_mut() {
        let v = *px;
        let r = ((v >> 16) & 0xFF) as u8;
        let g = ((v >> 8) & 0xFF) as u8;
        let b = (v & 0xFF) as u8;
        *px = pack_argb2101010((r, g, b));
    }
}

/// Blits an array of `TerminalCell`s directly into an ARGB8888 pixel buffer (`0xAARRGGBB`).
#[inline]
pub fn blit_cells_to_argb(
    cells: &[TerminalCell],
    cols: usize,
    rows: usize,
    buffer: &mut [u32],
    pixel_w: usize,
    pixel_h: usize,
) {
    if cols == 0 || rows == 0 || pixel_w == 0 || pixel_h == 0 {
        return;
    }

    let (char_w, char_h) = ((pixel_w / cols).max(1), (pixel_h / rows).max(1));
    let subcell_active = cells
        .iter()
        .take(cols.saturating_mul(rows))
        .any(|c| !c.bold && matches!(c.ch, '▀' | '▄'))
        && has_subcell_overlay(cols, rows);

    let mut row_lut_stack = [0usize; 512];
    let mut col_lut_stack = [0usize; 512];
    let (row_lut_heap, col_lut_heap);
    let (row_lut, col_lut): (&[usize], &[usize]) = if char_h <= 512 && char_w <= 512 {
        for (dy, slot) in row_lut_stack.iter_mut().enumerate().take(char_h) {
            *slot = ((dy * GLYPH_HEIGHT) / char_h).min(GLYPH_HEIGHT - 1);
        }
        for (dx, slot) in col_lut_stack.iter_mut().enumerate().take(char_w) {
            *slot = ((dx * GLYPH_WIDTH) / char_w).min(7);
        }
        (&row_lut_stack[..char_h], &col_lut_stack[..char_w])
    } else {
        row_lut_heap = (0..char_h)
            .map(|dy| ((dy * GLYPH_HEIGHT) / char_h).min(GLYPH_HEIGHT - 1))
            .collect::<Vec<_>>();
        col_lut_heap = (0..char_w)
            .map(|dx| ((dx * GLYPH_WIDTH) / char_w).min(7))
            .collect::<Vec<_>>();
        (&row_lut_heap[..], &col_lut_heap[..])
    };

    for cy in 0..rows {
        for cx in 0..cols {
            let Some(&cell) = cells.get(cy * cols + cx) else {
                break;
            };
            let hide_coarse = subcell_active && !cell.bold && matches!(cell.ch, '▀' | '▄');
            let fg_color = pack_argb(cell.fg);
            let bg_color = if hide_coarse {
                pack_argb(crate::ascii::sys_info::detect_system_theme().bg_rgb)
            } else {
                pack_argb(cell.bg)
            };
            let glyph = if hide_coarse {
                [0u8; GLYPH_HEIGHT]
            } else {
                get_glyph_bitmap(cell.ch)
            };
            let (start_px, start_py) = (cx * char_w, cy * char_h);

            for (dy, &glyph_row) in row_lut.iter().enumerate() {
                let py = start_py + dy;
                if py >= pixel_h {
                    break;
                }
                let (bits, row_offset) = (glyph[glyph_row], py * pixel_w);
                for (dx, &glyph_col) in col_lut.iter().enumerate() {
                    let px = start_px + dx;
                    if px >= pixel_w {
                        break;
                    }
                    if row_offset + px < buffer.len() {
                        buffer[row_offset + px] = if (bits & (0x80 >> glyph_col)) != 0 {
                            fg_color
                        } else {
                            bg_color
                        };
                    }
                }
            }
        }
    }

    let (drawn_w, drawn_h) = (cols * char_w, rows * char_h);
    if subcell_active {
        composite_subcell_overlay(
            cells,
            (cols, rows),
            buffer,
            (pixel_w, pixel_h),
            (drawn_w, drawn_h),
        );
    }

    let margin_bg = cells.first().map_or(0xFF00_0000, |c| pack_argb(c.bg));
    if drawn_h < pixel_h {
        let (start, end) = (drawn_h * pixel_w, (pixel_w * pixel_h).min(buffer.len()));
        if start < end {
            buffer[start..end].fill(margin_bg);
        }
    }
    if drawn_w < pixel_w {
        for y in 0..drawn_h.min(pixel_h) {
            let (rs, re) = (y * pixel_w + drawn_w, ((y + 1) * pixel_w).min(buffer.len()));
            if rs < re {
                buffer[rs..re].fill(margin_bg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blit_basic_and_zero_dimensions() {
        let (cols, rows) = (10, 5);
        let (pixel_w, pixel_h) = (cols * GLYPH_WIDTH, rows * GLYPH_HEIGHT);
        let mut buffer = vec![0u32; pixel_w * pixel_h];
        let mut cells = vec![TerminalCell::default(); cols * rows];
        blit_cells_to_argb(&cells, 0, 5, &mut buffer, 10, 10);
        blit_cells_to_argb(&cells, 5, 0, &mut buffer, 10, 10);
        blit_cells_to_argb(&cells, 5, 5, &mut buffer, 0, 10);
        blit_cells_to_argb(&cells, 5, 5, &mut buffer, 10, 0);
        assert_eq!(buffer[0], 0);

        cells[0].ch = '█';
        cells[0].fg = (255, 255, 255);
        blit_cells_to_argb(&cells, cols, rows, &mut buffer, pixel_w, pixel_h);
        assert_eq!(buffer[0], 0xFFFF_FFFF);
    }

    #[test]
    fn test_blit_scaled_extreme_and_margins() {
        let mut buf = vec![0u32; 80 * 120];
        let mut cells = vec![TerminalCell::default(); 16];
        cells[0].ch = '█';
        cells[0].fg = (200, 100, 50);
        blit_cells_to_argb(&cells, 4, 4, &mut buf, 80, 120);
        assert_eq!(buf[0], 0xFFC8_6432);
        assert_eq!(buf[80 * 15 + 10], 0xFFC8_6432);

        let mut ext = vec![0u32; 600 * 600];
        cells[0].fg = (255, 0, 0);
        blit_cells_to_argb(&cells[..1], 1, 1, &mut ext, 600, 600);
        assert_eq!(ext[0], 0xFFFF_0000);

        let mut marg = vec![0u32; 84 * 86];
        let grid = vec![TerminalCell::default(); 50];
        blit_cells_to_argb(&grid, 10, 5, &mut marg, 84, 86);
        assert_eq!(marg[85 * 84 + 5], 0xFF00_0000);
        assert_eq!(marg[10 * 84 + 82], 0xFF00_0000);
    }
}
