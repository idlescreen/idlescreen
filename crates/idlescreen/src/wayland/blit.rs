// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Direct CPU font blitter into wl_shm ARGB8888 buffers (<0.5ms frame time).

use crate::cell::TerminalCell;

pub const GLYPH_WIDTH: usize = 8;
pub const GLYPH_HEIGHT: usize = 16;

/// 8x16 1-bit pixel-art block font for full blocks, half-blocks, quadrants,
/// 1/8th bars, chunky 2x2 Bayer dithers, and retro pixel sprites.
fn get_glyph_bitmap(ch: char) -> [u8; GLYPH_HEIGHT] {
    match ch {
        ' ' => [0; 16],
        '█' => [0xFF; 16],
        '▀' => [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        '▄' => [
            0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ],
        '▌' => [0xF0; 16],
        '▐' => [0x0F; 16],
        '▖' => [
            0, 0, 0, 0, 0, 0, 0, 0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0,
        ],
        '▗' => [
            0, 0, 0, 0, 0, 0, 0, 0, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F,
        ],
        '▘' => [
            0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        '▝' => [
            0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        '▚' => [
            0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F,
            0x0F, 0x0F,
        ],
        '▞' => [
            0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0,
            0xF0, 0xF0,
        ],
        '▙' => [
            0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF,
        ],
        '▛' => [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0,
            0xF0, 0xF0,
        ],
        '▜' => [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F,
            0x0F, 0x0F,
        ],
        '▟' => [
            0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF,
        ],
        '▁' => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF],
        '▂' => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF],
        '▃' => [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ],
        '▅' => [
            0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ],
        '▆' => [
            0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ],
        '▇' => [
            0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF,
        ],
        '▏' => [0x80; 16],
        '▎' => [0xC0; 16],
        '▍' => [0xE0; 16],
        '▋' => [0xF8; 16],
        '▊' => [0xFC; 16],
        '▉' => [0xFE; 16],
        // 75% chunky 2x2 Bayer pixel dither
        '▓' => [
            0xFF, 0xFF, 0xCC, 0xCC, 0xFF, 0xFF, 0x33, 0x33, 0xFF, 0xFF, 0xCC, 0xCC, 0xFF, 0xFF,
            0x33, 0x33,
        ],
        // 50% chunky 2x2 checkerboard pixel dither
        '▒' => [
            0xCC, 0xCC, 0x33, 0x33, 0xCC, 0xCC, 0x33, 0x33, 0xCC, 0xCC, 0x33, 0x33, 0xCC, 0xCC,
            0x33, 0x33,
        ],
        // 25% chunky 2x2 sparse pixel dither
        '░' => [
            0xCC, 0xCC, 0x00, 0x00, 0x33, 0x33, 0x00, 0x00, 0xCC, 0xCC, 0x00, 0x00, 0x33, 0x33,
            0x00, 0x00,
        ],
        // Centered 6x10 pixel block
        '■' => [
            0, 0, 0, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0, 0, 0,
        ],
        // Centered 4x6 pixel sprite
        '.' | ':' => [
            0, 0, 0, 0, 0, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0, 0, 0, 0, 0,
        ],
        '│' | '|' => [0x3C; 16],
        '─' | '-' => [0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0],
        _ => [
            0, 0, 0, 0, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0, 0, 0, 0,
        ],
    }
}

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

    // Precompute glyph row and column coordinate lookup tables without heap allocations
    // Eliminates per-frame heap churn and millions of inner-loop divisions
    let mut row_lut_stack = [0usize; 512];
    let mut col_lut_stack = [0usize; 512];
    let (row_lut_heap, col_lut_heap);
    let (row_lut, col_lut): (&[usize], &[usize]) = if char_h <= 512 && char_w <= 512 {
        for dy in 0..char_h {
            row_lut_stack[dy] = ((dy * GLYPH_HEIGHT) / char_h).min(GLYPH_HEIGHT - 1);
        }
        for dx in 0..char_w {
            col_lut_stack[dx] = ((dx * GLYPH_WIDTH) / char_w).min(7);
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
                | ((cell.fg.0 as u32) << 16)
                | ((cell.fg.1 as u32) << 8)
                | (cell.fg.2 as u32);
            let bg_color: u32 = 0xFF00_0000
                | ((cell.bg.0 as u32) << 16)
                | ((cell.bg.1 as u32) << 8)
                | (cell.bg.2 as u32);

            let glyph = get_glyph_bitmap(cell.ch);

            let start_px = cx * char_w;
            let start_py = cy * char_h;

            for dy in 0..char_h {
                let py = start_py + dy;
                if py >= pixel_h {
                    break;
                }
                let glyph_row = row_lut[dy];
                let bits = glyph[glyph_row];
                let row_offset = py * pixel_w;

                for dx in 0..char_w {
                    let px = start_px + dx;
                    if px >= pixel_w {
                        break;
                    }
                    let glyph_col = col_lut[dx];
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

        // Check that top-left pixel is painted white (0xFFFFFFFF)
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
        let pixel_w = 80; // 20 px/char
        let pixel_h = 120; // 30 px/char
        let mut buffer = vec![0u32; pixel_w * pixel_h];
        let mut cells = vec![TerminalCell::default(); cols * rows];
        cells[0].ch = '█';
        cells[0].fg = (200, 100, 50);

        blit_cells_to_argb(&cells, cols, rows, &mut buffer, pixel_w, pixel_h);
        // Verify pixel within first cell has expected color (0xFFC86432)
        assert_eq!(buffer[0], 0xFFC8_6432);
        assert_eq!(buffer[pixel_w * 15 + 10], 0xFFC8_6432);
    }

    #[test]
    fn test_blit_extreme_resolution_fallback() {
        let cols = 1;
        let rows = 1;
        let pixel_w = 600; // > 512 triggers heap fallback
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
        // pixel dimensions that leave 4px right margin and 6px bottom margin
        let pixel_w = 84; // char_w = 8, 10 * 8 = 80, margin = 4
        let pixel_h = 86; // char_h = 16, 5 * 16 = 80, margin = 6
        let mut buffer = vec![0u32; pixel_w * pixel_h];
        let cells = vec![TerminalCell::default(); cols * rows];

        blit_cells_to_argb(&cells, cols, rows, &mut buffer, pixel_w, pixel_h);

        // Bottom margin row should be filled with opaque black 0xFF000000
        let bottom_pixel = buffer[85 * pixel_w + 5];
        assert_eq!(bottom_pixel, 0xFF00_0000);

        // Right margin column should be filled with opaque black 0xFF000000
        let right_pixel = buffer[10 * pixel_w + 82];
        assert_eq!(right_pixel, 0xFF00_0000);
    }
}
