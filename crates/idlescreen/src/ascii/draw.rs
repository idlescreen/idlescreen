// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal cell buffer drawing, 16-bit pixel-art quantization, arcade post-FX, and transitions.

use crate::cell::TerminalCell;

/// Returns true if `ch` is a pure pixel-art block/quadrant/dither element (or space).
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
            | '■'
            | '▪'
            | '▫'
    )
}

/// Maps any character emitted by a scene into a crisp 16-bit pixel-art block glyph.
/// Guarantees zero ASCII text letters, digits, katakana, or punctuation ever render.
#[inline]
pub fn pixel_art_glyph(ch: char, x: usize, y: usize) -> char {
    if is_pixel_art_char(ch) {
        return ch;
    }
    const HEAVY: [char; 6] = ['█', '▓', '▛', '▟', '▙', '▜'];
    const MEDIUM: [char; 12] = ['▓', '▒', '▀', '▄', '▌', '▐', '▛', '▟', '▙', '▜', '▞', '▚'];
    const SPARK: [char; 8] = ['■', '▪', '▀', '▄', '▌', '▐', '▞', '▚'];
    const FINE: [char; 6] = ['░', '▪', '▖', '▗', '▘', '▝'];

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
fn boost_arcade_rgb(c: (u8, u8, u8)) -> (u8, u8, u8) {
    let max_c = c.0.max(c.1).max(c.2);
    if max_c == 0 {
        return (56, 140, 235);
    }
    let min_c = c.0.min(c.1).min(c.2);
    // Boost luminance if too dim and stretch chroma contrast for 16-bit vibrancy
    let target_max = max_c.max(150) as u16;
    let scale = (target_max * 256) / (max_c as u16).max(1);
    let boost = |v: u8| -> u8 {
        let spread = (v.saturating_sub(min_c / 2) as u16 * scale) >> 8;
        spread.min(255) as u8
    };
    (boost(c.0), boost(c.1), boost(c.2))
}

/// Parses an ANSI-formatted ttfx frame string directly into the grid as pure pixel art.
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
        if let Some(raw_ch) = std::str::from_utf8(&bytes[i..end])
            .ok()
            .and_then(|s| s.chars().next())
        {
            if x < cols && y < rows {
                let idx = y * cols + x;
                if idx < grid.len() {
                    let ch = pixel_art_glyph(raw_ch, x, y);
                    let cell_fg = if ch == ' ' {
                        default_fg
                    } else {
                        boost_arcade_rgb(fg)
                    };
                    grid[idx] = TerminalCell {
                        ch,
                        fg: cell_fg,
                        bg,
                        bold: ch == '█' || bold,
                    };
                }
            }
            x += 1;
        }
        i = end;
    }
}

/// Applies 16-bit SNES arcade post-processing:
/// 1. 3D pixel-block drop shadow behind solid logo blocks
/// 2. Vibrant 16-bit bevel gradient and diagonal specular gleam sweep across settled logo blocks
/// 3. 3-layer scrolling parallax pixel starfield and rising neon arcade embers in empty space
pub fn apply_arcade_fx(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    frame_tick: u64,
    settled: bool,
    bg: (u8, u8, u8),
) {
    let total = cols.saturating_mul(rows).min(grid.len());
    if total == 0 || cols == 0 || rows == 0 {
        return;
    }

    let tick = frame_tick as usize;
    let gleam_span = (cols + rows * 2 + 28).max(1);
    let gleam_pos = (tick * 3) % gleam_span;

    // Pass 1: Metallic 16-bit bevel + specular gleam on solid blocks, plus 3D drop shadow
    for y in 0..rows {
        let row_off = y * cols;
        for x in 0..cols {
            let idx = row_off + x;
            if idx >= total {
                break;
            }
            let cell = grid[idx];
            if cell.ch == '█' || cell.ch == '▀' || cell.ch == '▄' {
                let diag = x + y * 2;
                let dist = diag.abs_diff(gleam_pos);
                let new_fg = if dist == 0 {
                    (245, 255, 255)
                } else if dist == 1 {
                    (255, 225, 110)
                } else if dist == 2 {
                    (130, 235, 255)
                } else if settled {
                    // Crisp 16-bit SuperFedora chrome-cyan to electric-sapphire vertical bevel
                    let band = (y * 16) / rows.max(1);
                    match band {
                        0..=5 => (255, 205, 70),   // Top "SUPER" arcade gold-cyan
                        6 => (150, 240, 255),      // Upper bevel highlight
                        7..=9 => (95, 195, 248),   // Sky cyan
                        10..=12 => (81, 162, 218), // Fedora cyan
                        _ => (58, 124, 220),       // Deep royal sapphire base
                    }
                } else {
                    cell.fg
                };
                grid[idx].fg = new_fg;
                grid[idx].bold = true;

                // Cast 3D 16-bit arcade shadow at (x + 1, y + 1) if empty
                if x + 1 < cols && y + 1 < rows {
                    let shadow_idx = (y + 1) * cols + (x + 1);
                    if shadow_idx < total && grid[shadow_idx].ch == ' ' {
                        grid[shadow_idx] = TerminalCell {
                            ch: '░',
                            fg: (36, 66, 138),
                            bg,
                            bold: false,
                        };
                    }
                }
            }
        }
    }

    // Pass 2: 3-layer scrolling 16-bit parallax pixel starfield in empty cells
    for y in 0..rows {
        let row_off = y * cols;
        for x in 0..cols {
            let idx = row_off + x;
            if idx >= total {
                break;
            }
            if grid[idx].ch != ' ' {
                continue;
            }

            // Layer 1: Fast horizontal pixel starstream
            let sx1 = x.wrapping_add(tick * 2);
            let h1 = (sx1.wrapping_mul(2_654_435_761) ^ y.wrapping_mul(2_246_822_519)) & 0xFFFF;
            if h1 < 72 {
                let is_gold = h1.trailing_zeros() >= 2;
                grid[idx] = TerminalCell {
                    ch: if (h1 & 1) == 0 { '▪' } else { '■' },
                    fg: if is_gold {
                        (255, 200, 60)
                    } else {
                        (95, 220, 255)
                    },
                    bg,
                    bold: true,
                };
                continue;
            }

            // Layer 2: Medium rising neon arcade embers
            let sy2 = y.wrapping_add(tick / 2);
            let h2 = (x.wrapping_mul(3_266_489_917) ^ sy2.wrapping_mul(668_265_263)) & 0xFFFF;
            if h2 < 96 {
                grid[idx] = TerminalCell {
                    ch: if (h2 & 1) == 0 { '▝' } else { '▖' },
                    fg: if (h2 & 2) == 0 {
                        (215, 85, 210)
                    } else {
                        (130, 95, 255)
                    },
                    bg,
                    bold: false,
                };
                continue;
            }

            // Layer 3: Slow deep-space pixel dither dust
            let sx3 = x.wrapping_add(tick / 3);
            let h3 = (sx3.wrapping_mul(374_761_393) ^ y.wrapping_mul(1_103_515_245)) & 0xFFFF;
            if h3 < 130 {
                grid[idx] = TerminalCell {
                    ch: '░',
                    fg: (40, 78, 156),
                    bg,
                    bold: false,
                };
            }
        }
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

/// Blends the outgoing frame and incoming frame over progress `t` in `[0.0, 1.0]`.
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

            let thresh = ((x.wrapping_mul(137) ^ y.wrapping_mul(149) ^ ((x + y).wrapping_mul(31)))
                & 0xFF) as u8;
            let pick_to = t >= 1.0 || (t > 0.0 && t_byte > thresh);
            let factor = if pick_to {
                200 + ((in_factor * 56) >> 8)
            } else {
                200 + ((out_factor * 56) >> 8)
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
