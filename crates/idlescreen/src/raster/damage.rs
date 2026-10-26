// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Dirty-rectangle computation for the cell grid.
//!
//! Diffing two cell grids to find the pixels that changed is presentation
//! math, not compositor work: it only compares `TerminalCell`s and does
//! arithmetic on the cell-to-pixel scale. It lives here so the raster tree
//! stays free of any product dependency, and so the blitter's own test can
//! exercise it without a Wayland session.

use crate::cell::TerminalCell;

/// Computes the tight pixel bounding box `(x, y, width, height)` between `prev` and `cells`,
/// returning `None` when zero cells changed.
pub fn compute_cell_damage_rect(
    prev: &[TerminalCell],
    cells: &[TerminalCell],
    (cols, rows): (usize, usize),
    (pixel_w, pixel_h): (usize, usize),
    force_full: bool,
) -> Option<(i32, i32, i32, i32)> {
    let total = cols.saturating_mul(rows);
    if pixel_w == 0 || pixel_h == 0 || cols == 0 || rows == 0 {
        return None;
    }
    let full = (
        0,
        0,
        i32::try_from(pixel_w).unwrap_or(i32::MAX),
        i32::try_from(pixel_h).unwrap_or(i32::MAX),
    );
    if force_full
        || prev.len() != total
        || cells.len() < total
        || prev.first().map(|c| c.bg) != cells.first().map(|c| c.bg)
    {
        return Some(full);
    }
    let (mut min_cx, mut max_cx) = (cols, 0usize);
    let (mut min_cy, mut max_cy) = (rows, 0usize);
    for cy in 0..rows {
        let row_off = cy * cols;
        for cx in 0..cols {
            if prev[row_off + cx] != cells[row_off + cx] {
                min_cx = min_cx.min(cx);
                max_cx = max_cx.max(cx);
                min_cy = min_cy.min(cy);
                max_cy = max_cy.max(cy);
            }
        }
    }
    if min_cx > max_cx {
        return None;
    }
    let (char_w, char_h) = ((pixel_w / cols).max(1), (pixel_h / rows).max(1));
    let px0 = (min_cx * char_w).min(pixel_w);
    let py0 = (min_cy * char_h).min(pixel_h);
    let px1 = ((max_cx + 1) * char_w).min(pixel_w);
    let py1 = ((max_cy + 1) * char_h).min(pixel_h);
    Some((
        i32::try_from(px0).unwrap_or(0),
        i32::try_from(py0).unwrap_or(0),
        i32::try_from(px1.saturating_sub(px0).max(1)).unwrap_or(i32::MAX),
        i32::try_from(py1.saturating_sub(py0).max(1)).unwrap_or(i32::MAX),
    ))
}
