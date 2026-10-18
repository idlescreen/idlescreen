// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Direct CPU font blitter into wl_shm ARGB8888 / ARGB2101010 buffers (<0.3ms frame time)
//! with branchless SIMD-friendly glyph row writes and dirty-cell partial damage tracking.

use crate::cell::TerminalCell;
pub use crate::wayland::glyph_font::{GLYPH_HEIGHT, GLYPH_WIDTH, get_glyph_bitmap};
pub use crate::wayland::subcell_overlay::{
    SUBCELL_X, SUBCELL_Y, clear_subcell_overlay, composite_subcell_overlay,
    composite_subcell_overlay_with_format, has_subcell_overlay, publish_subcell_overlay,
    release_subcell_overlay, subcell_overlay_generation,
};

#[inline(always)]
pub(super) fn pack_argb(c: (u8, u8, u8)) -> u32 {
    0xFF00_0000 | (u32::from(c.0) << 16) | (u32::from(c.1) << 8) | u32::from(c.2)
}

/// Packs 8-bit RGB channels into 10-bit `Argb2101010` (`[31:30] A=3, [29:20] R, [19:10] G, [9:0] B`).
#[inline(always)]
pub fn pack_argb2101010(c: (u8, u8, u8)) -> u32 {
    let expand10 = |v: u8| -> u32 { (u32::from(v) << 2) | (u32::from(v) >> 6) };
    0xC000_0000 | (expand10(c.0) << 20) | (expand10(c.1) << 10) | expand10(c.2)
}

#[inline(always)]
pub fn pack_color(c: (u8, u8, u8), is_10bit_hdr: bool) -> u32 {
    if is_10bit_hdr {
        pack_argb2101010(c)
    } else {
        pack_argb(c)
    }
}

/// Converts an in-place `Argb8888` pixel buffer into 10-bit `Argb2101010` for HDR/deep-color `wl_shm`.
#[inline]
pub fn convert_argb8888_to_2101010_in_place(buffer: &mut [u32]) {
    for px in buffer.iter_mut() {
        let v = *px;
        *px = pack_argb2101010((
            ((v >> 16) & 0xFF) as u8,
            ((v >> 8) & 0xFF) as u8,
            (v & 0xFF) as u8,
        ));
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
    blit_cells_with_format(cells, (cols, rows), buffer, (pixel_w, pixel_h), false, None);
}

#[inline(always)]
fn blit_glyph_row(row_slice: &mut [u32], bits: u8, col_lut: &[usize], fg: u32, bg: u32) {
    let xor_fg_bg = fg ^ bg;
    if col_lut.len() == 8 && row_slice.len() >= 8 {
        for (dx, slot) in row_slice[..8].iter_mut().enumerate() {
            *slot = bg ^ (xor_fg_bg & 0u32.wrapping_sub(u32::from((bits >> (7 - dx)) & 1)));
        }
    } else if col_lut.len() == 16 && row_slice.len() >= 16 {
        for (dx, slot) in row_slice[..16].iter_mut().enumerate() {
            *slot = bg ^ (xor_fg_bg & 0u32.wrapping_sub(u32::from((bits >> (7 - (dx >> 1))) & 1)));
        }
    } else {
        for (slot, &glyph_col) in row_slice.iter_mut().zip(col_lut.iter()) {
            *slot = bg ^ (xor_fg_bg & 0u32.wrapping_sub(u32::from((bits >> (7 - glyph_col)) & 1)));
        }
    }
}

/// Blits `cells` into `buffer` in 8-bit `Argb8888` or 10-bit `Argb2101010` in a single pass,
/// optionally skipping cells that match `prev_cells` when no subcell overlay is active.
#[inline]
pub fn blit_cells_with_format(
    cells: &[TerminalCell],
    (cols, rows): (usize, usize),
    buffer: &mut [u32],
    (pixel_w, pixel_h): (usize, usize),
    is_10bit_hdr: bool,
    prev_cells: Option<&[TerminalCell]>,
) -> bool {
    if cols == 0 || rows == 0 || pixel_w == 0 || pixel_h == 0 {
        return false;
    }

    let (char_w, char_h) = ((pixel_w / cols).max(1), (pixel_h / rows).max(1));
    let total_cells = cols.saturating_mul(rows);
    let subcell_active = cells
        .iter()
        .take(total_cells)
        .any(|c| !c.bold && matches!(c.ch, '▀' | '▄'))
        && has_subcell_overlay(cols, rows);
    let prev = (!subcell_active)
        .then_some(prev_cells)
        .flatten()
        .filter(|p| p.len() == total_cells);

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

    let theme_bg = crate::ascii::sys_info::detect_system_theme().bg_rgb;
    for cy in 0..rows {
        let row_idx = cy * cols;
        for cx in 0..cols {
            let idx = row_idx + cx;
            let Some(&cell) = cells.get(idx) else {
                break;
            };
            if prev.is_some_and(|p| p[idx] == cell) {
                continue;
            }
            let hide_coarse = subcell_active && !cell.bold && matches!(cell.ch, '▀' | '▄');
            let fg_color = pack_color(cell.fg, is_10bit_hdr);
            let bg_color = pack_color(if hide_coarse { theme_bg } else { cell.bg }, is_10bit_hdr);
            let glyph = if hide_coarse {
                [0u8; GLYPH_HEIGHT]
            } else {
                get_glyph_bitmap(cell.ch)
            };
            let (start_px, start_py) = (cx * char_w, cy * char_h);
            let end_px = (start_px + char_w).min(pixel_w);
            if start_px >= end_px {
                continue;
            }
            for (dy, &glyph_row) in row_lut.iter().enumerate() {
                let py = start_py + dy;
                if py >= pixel_h {
                    break;
                }
                let (rs, re) = (
                    py * pixel_w + start_px,
                    (py * pixel_w + end_px).min(buffer.len()),
                );
                if rs < re {
                    blit_glyph_row(
                        &mut buffer[rs..re],
                        glyph[glyph_row],
                        col_lut,
                        fg_color,
                        bg_color,
                    );
                }
            }
        }
    }

    let (drawn_w, drawn_h) = (cols * char_w, rows * char_h);
    if subcell_active {
        composite_subcell_overlay_with_format(
            cells,
            (cols, rows),
            buffer,
            (pixel_w, pixel_h),
            (drawn_w, drawn_h),
            is_10bit_hdr,
        );
    }

    if prev.is_none_or(|p| p.first().map(|c| c.bg) != cells.first().map(|c| c.bg)) {
        let margin_bg = pack_color(cells.first().map_or((0, 0, 0), |c| c.bg), is_10bit_hdr);
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
    subcell_active
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::presenter::compute_cell_damage_rect;

    #[test]
    fn test_blit_basic_scaled_hdr_and_dirty_diff() {
        let (cols, rows) = (10, 5);
        let (pixel_w, pixel_h) = (cols * GLYPH_WIDTH, rows * GLYPH_HEIGHT);
        let mut buffer = vec![0u32; pixel_w * pixel_h];
        let mut cells = vec![TerminalCell::default(); cols * rows];
        blit_cells_to_argb(&cells, 0, 5, &mut buffer, 10, 10);
        blit_cells_to_argb(&cells, 5, 0, &mut buffer, 10, 10);
        blit_cells_to_argb(&cells, 5, 5, &mut buffer, 0, 10);
        blit_cells_to_argb(&cells, 5, 5, &mut buffer, 10, 0);
        assert_eq!(buffer[0], 0);

        (cells[0].ch, cells[0].fg) = ('█', (255, 255, 255));
        blit_cells_to_argb(&cells, cols, rows, &mut buffer, pixel_w, pixel_h);
        assert_eq!(buffer[0], 0xFFFF_FFFF);

        let mut ext = vec![0u32; 600 * 600];
        cells[0].fg = (255, 0, 0);
        blit_cells_to_argb(&cells[..1], 1, 1, &mut ext, 600, 600);
        assert_eq!(ext[0], 0xFFFF_0000);

        let mut marg = vec![0u32; 84 * 86];
        let grid = vec![TerminalCell::default(); 50];
        blit_cells_to_argb(&grid, 10, 5, &mut marg, 84, 86);
        assert_eq!(marg[85 * 84 + 5], 0xFF00_0000);
        assert_eq!(marg[10 * 84 + 82], 0xFF00_0000);
        blit_cells_with_format(&grid, (10, 5), &mut marg, (84, 86), true, None);
        assert_eq!(marg[85 * 84 + 5], 0xC000_0000);

        let mut next = grid.clone();
        let dmg = |a: &[TerminalCell], b: &[TerminalCell]| {
            compute_cell_damage_rect(a, b, (10, 5), (80, 80), false)
        };
        assert_eq!(dmg(&grid, &next), None);
        (next[2 * 10 + 3].ch, next[2 * 10 + 3].fg) = ('█', (255, 255, 255));
        assert_eq!(dmg(&grid, &next), Some((24, 32, 8, 16)));
        let mut diff_buf = vec![0xDEAD_BEEFu32; 80 * 80];
        blit_cells_with_format(&next, (10, 5), &mut diff_buf, (80, 80), false, Some(&grid));
        assert_eq!(
            (diff_buf[0], diff_buf[32 * 80 + 24]),
            (0xDEAD_BEEF, 0xFFFF_FFFF)
        );
    }
}
