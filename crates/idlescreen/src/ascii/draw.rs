// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal cell buffer drawing, ANSI frame parsing, and dissolve transitions.

use crate::cell::TerminalCell;

/// Parses an ANSI-formatted ttfx frame string directly into the grid.
///
/// Steady-state zero heap allocation: parses bytes in-place.
pub fn paint_frame(
    frame: &str,
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    default_fg: (u8, u8, u8),
    default_bg: (u8, u8, u8),
) {
    grid.fill(TerminalCell {
        ch: ' ',
        fg: default_fg,
        bg: default_bg,
        bold: false,
    });
    if cols == 0 || rows == 0 {
        return;
    }
    let (mut x, mut y, mut bold) = (0usize, 0usize, false);
    let (mut fg, mut bg) = (default_fg, default_bg);

    let bytes = frame.as_bytes();
    let mut i = 0;
    while i < bytes.len() && y < rows {
        if bytes[i] == b'\n' {
            y += 1;
            x = 0;
            i += 1;
            continue;
        }
        if bytes[i] == 0x1b {
            i += 1;
            if i < bytes.len() && bytes[i] == b'[' {
                i += 1;
                let start = i;
                while i < bytes.len() && (bytes[i] < b'@' || bytes[i] > b'~') {
                    i += 1;
                }
                if i < bytes.len() {
                    if bytes[i] == b'm' {
                        let seq = &bytes[start..i];
                        parse_sgr(seq, &mut fg, &mut bg, &mut bold, default_fg, default_bg);
                    }
                    i += 1;
                }
            } else if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        let lead = bytes[i];
        let len = if lead < 0x80 {
            1
        } else if lead < 0xe0 {
            2
        } else if lead < 0xf0 {
            3
        } else {
            4
        };
        let end = (i + len).min(bytes.len());
        if let Some(ch) = std::str::from_utf8(&bytes[i..end])
            .ok()
            .and_then(|s| s.chars().next())
        {
            if x < cols && y < rows {
                let idx = y * cols + x;
                if idx < grid.len() {
                    grid[idx] = TerminalCell { ch, fg, bg, bold };
                }
            }
            x += 1;
        }
        i = end;
    }
}

fn parse_sgr(
    seq: &[u8],
    fg: &mut (u8, u8, u8),
    bg: &mut (u8, u8, u8),
    bold: &mut bool,
    default_fg: (u8, u8, u8),
    default_bg: (u8, u8, u8),
) {
    if seq.is_empty() || seq == b"0" {
        *fg = default_fg;
        *bg = default_bg;
        *bold = false;
        return;
    }
    let mut parts = seq.split(|&b| b == b';');
    while let Some(part) = parts.next() {
        match part {
            b"0" => {
                *fg = default_fg;
                *bg = default_bg;
                *bold = false;
            }
            b"1" => *bold = true,
            b"22" => *bold = false,
            [b'3', d @ b'0'..=b'7'] => *fg = xterm_to_rgb(d - b'0'),
            [b'9', d @ b'0'..=b'7'] => *fg = xterm_to_rgb(8 + (d - b'0')),
            [b'4', d @ b'0'..=b'7'] => *bg = xterm_to_rgb(d - b'0'),
            [b'1', b'0', d @ b'0'..=b'7'] => *bg = xterm_to_rgb(8 + (d - b'0')),
            b"38" | b"48" => {
                let is_fg = part == b"38";
                let def = if is_fg { default_fg } else { default_bg };
                let target = if is_fg { &mut *fg } else { &mut *bg };
                match parts.next() {
                    Some(b"2") => {
                        let r = parts.next().and_then(parse_u8).unwrap_or(def.0);
                        let g = parts.next().and_then(parse_u8).unwrap_or(def.1);
                        let b = parts.next().and_then(parse_u8).unwrap_or(def.2);
                        *target = (r, g, b);
                    }
                    Some(b"5") => {
                        if let Some(code) = parts.next().and_then(parse_u8) {
                            *target = xterm_to_rgb(code);
                        }
                    }
                    _ => {}
                }
            }
            b"39" => *fg = default_fg,
            b"49" => *bg = default_bg,
            _ => {}
        }
    }
}

fn parse_u8(bytes: &[u8]) -> Option<u8> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

pub(crate) fn xterm_to_rgb(code: u8) -> (u8, u8, u8) {
    static TBL: std::sync::OnceLock<[(u8, u8, u8); 256]> = std::sync::OnceLock::new();
    TBL.get_or_init(|| {
        let mut t = [(255, 255, 255); 256];
        for (i, slot) in t.iter_mut().enumerate() {
            let hex = ttfx::utils::hexterm::xterm_to_hex(i as u8);
            if hex.len() == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                *slot = (r, g, b);
            }
        }
        t
    })[code as usize]
}

/// Smoothly blends the outgoing frame and incoming frame over progress `t` in `[0.0, 1.0]`.
///
/// Implements an energy-preserving dissolve transition:
/// - Eliminates all hard cuts, screen blanking, pops, and flashes.
/// - Outgoing settled text gracefully fades in luminance and dissolves.
/// - Incoming effect animation ramps in brightness and enters cleanly.
/// - Zero heap allocation, bounds-checked.
pub fn paint_transition(
    from: &[TerminalCell],
    to: &[TerminalCell],
    out: &mut [TerminalCell],
    t: f32,
    cols: usize,
    rows: usize,
) {
    let t = t.clamp(0.0, 1.0);
    let total = cols * rows;
    let n = total.min(from.len()).min(to.len()).min(out.len());
    if n == 0 {
        return;
    }

    let inv_t = 1.0 - t;
    let out_factor = (inv_t * 256.0) as u16;
    let in_factor = (t * 256.0) as u16;
    let t_byte = (t * 255.0) as u8;

    for y in 0..rows {
        let row_offset = y * cols;
        for x in 0..cols {
            let idx = row_offset + x;
            if idx >= n {
                break;
            }
            let from_cell = from[idx];
            let to_cell = to[idx];

            // Pseudo-random pseudo-dither threshold based on coordinates
            let thresh = ((x.wrapping_mul(137) ^ y.wrapping_mul(149) ^ ((x + y).wrapping_mul(31)))
                & 0xFF) as u8;
            let pick_to = t >= 1.0 || (t > 0.0 && t_byte > thresh);
            let factor = if pick_to {
                180 + ((in_factor * 76) >> 8)
            } else {
                180 + ((out_factor * 76) >> 8)
            };
            let src = if pick_to { to_cell } else { from_cell };
            out[idx] = if src.ch == ' ' {
                TerminalCell::default()
            } else {
                TerminalCell {
                    ch: src.ch,
                    fg: scale_rgb(src.fg, factor),
                    bg: src.bg,
                    bold: src.bold,
                }
            };
        }
    }

    if out.len() > n {
        out[n..].fill(TerminalCell::default());
    }
}

#[inline(always)]
fn scale_rgb(c: (u8, u8, u8), factor_256: u16) -> (u8, u8, u8) {
    (
        ((c.0 as u16 * factor_256) >> 8) as u8,
        ((c.1 as u16 * factor_256) >> 8) as u8,
        ((c.2 as u16 * factor_256) >> 8) as u8,
    )
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
                        ch,
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
