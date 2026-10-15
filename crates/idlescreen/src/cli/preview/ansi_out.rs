// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal ANSI cell-grid formatting and frame dispatch for preview sessions.

use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::time::Duration;

use crate::ascii::Ascii;
use crate::cell::TerminalCell;

pub fn draw_frame(
    dt: Duration,
    saver: &mut Ascii,
    grid: &mut [TerminalCell],
    (cols, rows): (usize, usize),
    tour_mode: bool,
    out: &mut std::io::Stdout,
) -> Result<(), Box<dyn std::error::Error>> {
    saver.update(dt, cols, rows);
    saver.draw(grid, cols, rows);
    crate::wayland::glyph_font::draw_preview_hud(grid, cols, rows, saver.effect, tour_mode);
    render_grid_to_terminal(out, grid, cols, rows)?;
    Ok(())
}

pub fn render_grid_to_terminal(
    out: &mut std::io::Stdout,
    grid: &[TerminalCell],
    cols: usize,
    rows: usize,
) -> std::io::Result<()> {
    let mut buf = String::with_capacity(cols * rows * 4);
    let mut cur_fg: Option<(u8, u8, u8)> = None;
    let mut cur_bg: Option<(u8, u8, u8)> = None;
    let mut cur_bold: Option<bool> = None;

    for y in 0..rows {
        let _ = write!(buf, "\x1b[{};1H", y + 1);
        let row_offset = y * cols;
        for x in 0..cols {
            let idx = row_offset + x;
            if idx < grid.len() {
                let cell = grid[idx];
                if cur_bold != Some(cell.bold) {
                    cur_bold = Some(cell.bold);
                    buf.push_str(if cell.bold { "\x1b[1m" } else { "\x1b[22m" });
                }
                if cur_fg != Some(cell.fg) {
                    cur_fg = Some(cell.fg);
                    let (r, g, b) = cell.fg;
                    let _ = write!(buf, "\x1b[38;2;{r};{g};{b}m");
                }
                if cur_bg != Some(cell.bg) {
                    cur_bg = Some(cell.bg);
                    let (r, g, b) = cell.bg;
                    let _ = write!(buf, "\x1b[48;2;{r};{g};{b}m");
                }
                buf.push(cell.ch);
            }
        }
    }
    buf.push_str("\x1b[0m");
    let _ = write!(out, "{buf}");
    out.flush()
}
