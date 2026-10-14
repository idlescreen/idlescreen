// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Direct CPU font blitter into wl_shm ARGB8888 buffers (<0.5ms frame time).

use crate::cell::TerminalCell;
pub use crate::wayland::glyph_font::{GLYPH_HEIGHT, GLYPH_WIDTH, get_glyph_bitmap};

/// Blits an array of `TerminalCell`s directly into an ARGB8888 pixel buffer.
///
/// Output buffer format is native 32-bit ARGB8888 (`0xAARRGGBB`).
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

    let char_w = (pixel_w / cols).max(1);
    let char_h = (pixel_h / rows).max(1);

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
            let cell_idx = cy * cols + cx;
            if cell_idx >= cells.len() {
                break;
            }
            let cell = cells[cell_idx];

            let fg_color: u32 = 0xFF00_0000
                | (u32::from(cell.fg.0) << 16)
                | (u32::from(cell.fg.1) << 8)
                | u32::from(cell.fg.2);
            let bg_color: u32 = 0xFF00_0000
                | (u32::from(cell.bg.0) << 16)
                | (u32::from(cell.bg.1) << 8)
                | u32::from(cell.bg.2);

            let glyph = get_glyph_bitmap(cell.ch);
            let start_px = cx * char_w;
            let start_py = cy * char_h;

            for (dy, &glyph_row) in row_lut.iter().enumerate() {
                let py = start_py + dy;
                if py >= pixel_h {
                    break;
                }
                let bits = glyph[glyph_row];
                let row_offset = py * pixel_w;

                for (dx, &glyph_col) in col_lut.iter().enumerate() {
                    let px = start_px + dx;
                    if px >= pixel_w {
                        break;
                    }
                    let is_fg = (bits & (0x80 >> glyph_col)) != 0;
                    let color = if is_fg { fg_color } else { bg_color };
                    let pixel_idx = row_offset + px;
                    if pixel_idx < buffer.len() {
                        buffer[pixel_idx] = color;
                    }
                }
            }
        }
    }

    let drawn_w = cols * char_w;
    let drawn_h = rows * char_h;
    if drawn_h < pixel_h {
        let start = drawn_h * pixel_w;
        let end = (pixel_w * pixel_h).min(buffer.len());
        if start < end {
            buffer[start..end].fill(0xFF00_0000);
        }
    }
    if drawn_w < pixel_w {
        for y in 0..drawn_h.min(pixel_h) {
            let row_start = y * pixel_w + drawn_w;
            let row_end = ((y + 1) * pixel_w).min(buffer.len());
            if row_start < row_end {
                buffer[row_start..row_end].fill(0xFF00_0000);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blit_basic_grid() {
        let cols = 10;
        let rows = 5;
        let pixel_w = cols * GLYPH_WIDTH;
        let pixel_h = rows * GLYPH_HEIGHT;
        let mut buffer = vec![0u32; pixel_w * pixel_h];
        let mut cells = vec![TerminalCell::default(); cols * rows];

        cells[0].ch = '█';
        cells[0].fg = (255, 255, 255);

        blit_cells_to_argb(&cells, cols, rows, &mut buffer, pixel_w, pixel_h);
        assert_eq!(buffer[0], 0xFFFF_FFFF);
    }

    #[test]
    fn test_blit_zero_dimensions() {
        let mut buffer = vec![0u32; 100];
        let cells = vec![TerminalCell::default(); 10];
        blit_cells_to_argb(&cells, 0, 5, &mut buffer, 10, 10);
        blit_cells_to_argb(&cells, 5, 0, &mut buffer, 10, 10);
        blit_cells_to_argb(&cells, 5, 5, &mut buffer, 0, 10);
        blit_cells_to_argb(&cells, 5, 5, &mut buffer, 10, 0);
        assert_eq!(buffer[0], 0);
    }

    #[test]
    fn test_blit_scaled_dimensions() {
        let cols = 4;
        let rows = 4;
        let pixel_w = 80;
        let pixel_h = 120;
        let mut buffer = vec![0u32; pixel_w * pixel_h];
        let mut cells = vec![TerminalCell::default(); cols * rows];
        cells[0].ch = '█';
        cells[0].fg = (200, 100, 50);

        blit_cells_to_argb(&cells, cols, rows, &mut buffer, pixel_w, pixel_h);
        assert_eq!(buffer[0], 0xFFC8_6432);
        assert_eq!(buffer[pixel_w * 15 + 10], 0xFFC8_6432);
    }

    #[test]
    fn test_blit_extreme_resolution_fallback() {
        let cols = 1;
        let rows = 1;
        let pixel_w = 600;
        let pixel_h = 600;
        let mut buffer = vec![0u32; pixel_w * pixel_h];
        let mut cells = vec![TerminalCell::default(); 1];
        cells[0].ch = '█';
        cells[0].fg = (255, 0, 0);

        blit_cells_to_argb(&cells, cols, rows, &mut buffer, pixel_w, pixel_h);
        assert_eq!(buffer[0], 0xFFFF_0000);
    }

    #[test]
    fn test_blit_margin_filling() {
        let cols = 10;
        let rows = 5;
        let pixel_w = 84;
        let pixel_h = 86;
        let mut buffer = vec![0u32; pixel_w * pixel_h];
        let cells = vec![TerminalCell::default(); cols * rows];

        blit_cells_to_argb(&cells, cols, rows, &mut buffer, pixel_w, pixel_h);
        assert_eq!(buffer[85 * pixel_w + 5], 0xFF00_0000);
        assert_eq!(buffer[10 * pixel_w + 82], 0xFF00_0000);
    }
}
