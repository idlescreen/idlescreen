// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal cell buffer drawing, 16-bit pixel-art quantization, arcade post-FX, and transitions.

mod ansi;
mod post_fx;

pub use ansi::paint_frame;
pub use post_fx::{apply_arcade_fx, apply_arcade_fx_with_bounds, paint_transition};

use crate::cell::TerminalCell;

/// Returns true if `ch` is a pure Unicode Block Element (`U+2580..=U+259F`) or space.
#[inline]
pub fn is_pixel_art_char(ch: char) -> bool {
    matches!(
        ch,
        ' ' | '█'
            | '▀'
            | '▄'
            | '▌'
            | '▐'
            | '▁'
            | '▂'
            | '▃'
            | '▅'
            | '▆'
            | '▇'
            | '▏'
            | '▎'
            | '▍'
            | '▋'
            | '▊'
            | '▉'
            | '░'
            | '▒'
            | '▓'
            | '▖'
            | '▗'
            | '▘'
            | '▝'
            | '▚'
            | '▞'
            | '▙'
            | '▛'
            | '▜'
            | '▟'
    )
}

/// Maps any character emitted by a scene into a crisp 16-bit pixel-art block glyph.
#[inline]
pub fn pixel_art_glyph(ch: char, x: usize, y: usize) -> char {
    if is_pixel_art_char(ch) {
        return ch;
    }
    const HEAVY: [char; 5] = ['▓', '▛', '▟', '▙', '▜'];
    const MEDIUM: [char; 12] = ['▓', '▒', '▀', '▄', '▌', '▐', '▛', '▟', '▙', '▜', '▞', '▚'];
    const SPARK: [char; 8] = ['▀', '▄', '▌', '▐', '▞', '▚', '▘', '▝'];
    const FINE: [char; 5] = ['░', '▖', '▗', '▘', '▝'];

    let hash = (ch as usize)
        .wrapping_mul(31)
        .wrapping_add(x.wrapping_mul(17))
        .wrapping_add(y.wrapping_mul(13));

    match ch {
        '@' | '#' | '%' | '&' | 'W' | 'M' | 'B' | '8' | '0' | 'Q' => HEAVY[hash % HEAVY.len()],
        '.' | ',' | '\'' | '`' | ':' | ';' | '-' | '_' | '~' => FINE[hash % FINE.len()],
        '*' | '+' | 'o' | 'O' | '^' | '/' | '\\' | '|' | '=' => SPARK[hash % SPARK.len()],
        _ => MEDIUM[hash % MEDIUM.len()],
    }
}

/// Boosts dull/dark RGB values into punchy, high-saturation 16-bit arcade colors.
#[inline]
pub(crate) fn boost_arcade_rgb(c: (u8, u8, u8)) -> (u8, u8, u8) {
    let max_c = c.0.max(c.1).max(c.2);
    if max_c == 0 {
        return (56, 140, 235);
    }
    let min_c = c.0.min(c.1).min(c.2);
    let target_max = max_c.max(150) as u16;
    let scale = (target_max * 256) / (max_c as u16).max(1);
    let boost = |v: u8| -> u8 {
        let spread = (v.saturating_sub(min_c / 2) as u16 * scale) >> 8;
        spread.min(255) as u8
    };
    (boost(c.0), boost(c.1), boost(c.2))
}

/// Computes the 16-bit SNES vertical chrome bevel color for a solid banner block at row `y`
/// relative to the banner's vertical bounding box `[min_y, max_y]`.
/// Only applies the 2-tier gold (`SUPER`) + cyan (`FEDORA`) split when `is_stacked_superfedora`
/// is true so single-line scaled banners on standard distros are never split mid-glyph (B3).
#[inline]
pub fn banner_bevel_color(
    y: usize,
    min_y: usize,
    max_y: usize,
    is_stacked_superfedora: bool,
) -> (u8, u8, u8) {
    let span = max_y.saturating_sub(min_y) + 1;
    let rel_y = y.saturating_sub(min_y);
    let super_rows = if !is_stacked_superfedora {
        0
    } else if span >= 18 {
        6
    } else if span >= 6 {
        (span * 3) / 7
    } else {
        0
    };

    if super_rows > 0 && rel_y < super_rows {
        let band = (rel_y * 6) / super_rows.max(1);
        match band {
            0 | 1 => (255, 240, 130),
            2 | 3 => (255, 205, 55),
            _ => (235, 155, 30),
        }
    } else {
        let fedora_start = if super_rows > 0 { super_rows + 1 } else { 0 };
        let fedora_span = span.saturating_sub(fedora_start).max(1);
        let fedora_y = rel_y.saturating_sub(fedora_start);
        let band = (fedora_y * 10) / fedora_span;
        match band {
            0 | 1 => (170, 245, 255),
            2..=4 => (100, 205, 255),
            5..=7 => (81, 162, 218),
            _ => (52, 118, 220),
        }
    }
}

/// Finds the vertical row bounds `(min_y, max_y)` of solid banner blocks (`█`, `▀`, `▄`).
pub fn find_solid_banner_bounds(
    grid: &[TerminalCell],
    cols: usize,
    rows: usize,
) -> Option<(usize, usize)> {
    let total = cols.saturating_mul(rows).min(grid.len());
    let mut min_y = rows;
    let mut max_y = 0usize;
    let mut found = false;
    for y in 0..rows {
        let row_off = y * cols;
        for x in 0..cols {
            let idx = row_off + x;
            if idx >= total {
                break;
            }
            let ch = grid[idx].ch;
            if ch == '█' || ch == '▀' || ch == '▄' {
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }
    found.then_some((min_y, max_y))
}

/// Stamps the 16-bit SNES vertical chrome bevel onto settled solid banner blocks in `grid`.
pub fn apply_settled_bevel(grid: &mut [TerminalCell], cols: usize, rows: usize) {
    let _ = apply_settled_bevel_for(grid, cols, rows, false);
}

/// Stamps the 16-bit SNES vertical chrome bevel onto settled solid banner blocks in `grid`,
/// returning the detected `(min_y, max_y)` bounds for zero-rescan dwell caching.
pub fn apply_settled_bevel_for(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    is_stacked_superfedora: bool,
) -> Option<(usize, usize)> {
    let (min_y, max_y) = find_solid_banner_bounds(grid, cols, rows)?;
    let total = cols.saturating_mul(rows).min(grid.len());
    for y in min_y..=max_y {
        let color = banner_bevel_color(y, min_y, max_y, is_stacked_superfedora);
        let row_off = y * cols;
        for x in 0..cols {
            let idx = row_off + x;
            if idx >= total {
                break;
            }
            let ch = grid[idx].ch;
            if ch == '█' || ch == '▀' || ch == '▄' {
                grid[idx].fg = color;
                grid[idx].bold = true;
            }
        }
    }
    Some((min_y, max_y))
}

/// Blits banner lines onto a target grid centered horizontally and vertically.
pub fn place_banner_centered(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    lines: &[String],
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) -> (usize, usize, usize, usize) {
    if cols == 0 || rows == 0 || lines.is_empty() {
        return (0, 0, 0, 0);
    }
    let banner_h = lines.len();
    let banner_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let start_x = cols.saturating_sub(banner_w) / 2;
    let start_y = rows.saturating_sub(banner_h) / 2;

    for (row_idx, line) in lines.iter().enumerate() {
        let y = start_y + row_idx;
        if y >= rows {
            break;
        }
        for (col_idx, ch) in line.chars().enumerate() {
            let x = start_x + col_idx;
            if x >= cols {
                break;
            }
            if ch != ' ' {
                let idx = y * cols + x;
                if idx < grid.len() {
                    grid[idx] = TerminalCell {
                        ch: pixel_art_glyph(ch, x, y),
                        fg,
                        bg,
                        bold: true,
                    };
                }
            }
        }
    }
    (start_x, start_y, banner_w, banner_h)
}
