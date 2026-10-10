// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Direct CPU font blitter into wl_shm ARGB8888 buffers (<0.5ms frame time).

use crate::cell::TerminalCell;

pub const GLYPH_WIDTH: usize = 8;
pub const GLYPH_HEIGHT: usize = 16;

/// 8x16 1-bit bitmap font for basic ASCII (0x20..=0x7E) and common block elements.
fn get_glyph_bitmap(ch: char) -> [u8; GLYPH_HEIGHT] {
    match ch {
        ' ' => [0; 16],
        '█' => [0xFF; 16],
        '▀' => [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        '▄' => [
            0, 0, 0, 0, 0, 0, 0, 0,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ],
        '▌' => [0xF0; 16],
        '▐' => [0x0F; 16],
        '│' | '|' => [0x18; 16],
        '─' | '-' => [
            0, 0, 0, 0, 0, 0, 0, 0xFF,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        '.' => [
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0x18, 0x18, 0, 0,
        ],
        ':' => [
            0, 0, 0, 0, 0x18, 0x18, 0, 0,
            0, 0, 0x18, 0x18, 0, 0, 0, 0,
        ],
        '!' => [
            0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0,
            0, 0x18, 0x18, 0, 0, 0, 0, 0,
        ],
        '?' => [
            0x3C, 0x66, 0x06, 0x0C, 0x18, 0x18, 0, 0x18,
            0x18, 0, 0, 0, 0, 0, 0, 0,
        ],
        '*' => [
            0, 0, 0x24, 0x18, 0x7E, 0x18, 0x24, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        '#' => [
            0, 0x24, 0x24, 0x7E, 0x24, 0x7E, 0x24, 0x24,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        '$' => [
            0x10, 0x3C, 0x52, 0x50, 0x38, 0x14, 0x52, 0x3C,
            0x10, 0, 0, 0, 0, 0, 0, 0,
        ],
        '0' => [
            0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        '1' => [
            0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        '2' => [
            0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'A' | 'a' => [
            0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'B' | 'b' => [
            0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'C' | 'c' => [
            0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'D' | 'd' => [
            0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'E' | 'e' => [
            0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x7E, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'F' | 'f' => [
            0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'I' | 'i' => [
            0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'L' | 'l' => [
            0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'N' | 'n' => [
            0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'O' | 'o' => [
            0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'R' | 'r' => [
            0x7C, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'S' | 's' => [
            0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'U' | 'u' => [
            0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        'X' | 'x' => [
            0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
        _ => [
            0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
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
                let glyph_row = (dy * GLYPH_HEIGHT) / char_h;
                let bits = glyph[glyph_row.min(GLYPH_HEIGHT - 1)];
                let row_offset = py * pixel_w;

                for dx in 0..char_w {
                    let px = start_px + dx;
                    if px >= pixel_w {
                        break;
                    }
                    let glyph_col = (dx * GLYPH_WIDTH) / char_w;
                    let is_fg = (bits & (0x80 >> glyph_col.min(7))) != 0;
                    let color = if is_fg { fg_color } else { bg_color };
                    let pixel_idx = row_offset + px;
                    if pixel_idx < buffer.len() {
                        buffer[pixel_idx] = color;
                    }
                }
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
}
