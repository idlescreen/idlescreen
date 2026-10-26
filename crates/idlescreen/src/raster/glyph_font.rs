// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 8x16 1-bit pixel-art block font bitmaps for full blocks, half-blocks, quadrants,
//! 1/8th bars, chunky 2x2 Bayer dithers, and 16-bit arcade HUD glyphs.

use crate::ascii::effects::EffectKind;
use crate::ascii::sys_info::detect_system_theme;
use crate::cell::TerminalCell;
use crate::raster::glyph_style::GlyphFontStyle;

pub const GLYPH_WIDTH: usize = 8;
pub const GLYPH_HEIGHT: usize = 16;

#[inline]
pub fn get_glyph_bitmap(ch: char) -> [u8; GLYPH_HEIGHT] {
    get_glyph_bitmap_for_style(ch, detect_system_theme().font_style)
}

#[inline]
#[rustfmt::skip]
pub fn get_glyph_bitmap_for_style(ch: char, style: GlyphFontStyle) -> [u8; GLYPH_HEIGHT] {
    match ch {
        ' ' => [0; 16],
        '█' => match style {
            GlyphFontStyle::ArcadeBlock => [0xFF; 16],
            GlyphFontStyle::CyberSharp => [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            GlyphFontStyle::RetroRound => [0x7E, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7E],
        },
        '▀' => [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0],
        '▄' => [0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        '▌' => [0xF0; 16],
        '▐' => [0x0F; 16],
        '▖' => [0, 0, 0, 0, 0, 0, 0, 0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0],
        '▗' => [0, 0, 0, 0, 0, 0, 0, 0, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F],
        '▘' => [0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0, 0, 0, 0, 0, 0, 0, 0],
        '▝' => [0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0, 0, 0, 0, 0, 0, 0, 0],
        '▚' => [0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F],
        '▞' => [0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0],
        '▙' => [0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        '▛' => [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0xF0],
        '▜' => [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F],
        '▟' => [0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        '▁' => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF],
        '▂' => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF],
        '▃' => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        '▅' => [0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        '▆' => [0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        '▇' => [0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        '▏' => [0x80; 16],
        '▎' => [0xC0; 16],
        '▍' => [0xE0; 16],
        '▋' => [0xF8; 16],
        '▊' => [0xFC; 16],
        '▉' => [0xFE; 16],
        '▓' => [0xFF, 0xFF, 0xCC, 0xCC, 0xFF, 0xFF, 0x33, 0x33, 0xFF, 0xFF, 0xCC, 0xCC, 0xFF, 0xFF, 0x33, 0x33],
        '▒' => [0xCC, 0xCC, 0x33, 0x33, 0xCC, 0xCC, 0x33, 0x33, 0xCC, 0xCC, 0x33, 0x33, 0xCC, 0xCC, 0x33, 0x33],
        '░' => [0xCC, 0xCC, 0x00, 0x00, 0x33, 0x33, 0x00, 0x00, 0xCC, 0xCC, 0x00, 0x00, 0x33, 0x33, 0x00, 0x00],
        '■' => [0, 0, 0, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x18, 0x18, 0, 0, 0],
        ':' => [0, 0, 0, 0, 0x18, 0x18, 0, 0, 0, 0, 0x18, 0x18, 0, 0, 0, 0],
        '│' | '|' => [0x18; 16],
        '─' | '-' => [0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0],
        other => expand_8x8_glyph(hud_8x8_glyph(other.to_ascii_uppercase())),
    }
}

#[inline]
fn expand_8x8_glyph(packed: u64) -> [u8; 16] {
    let mut out = [0u8; 16];
    for y in 2..14 {
        let row = ((y - 2) * 8) / 12;
        out[y] = ((packed >> ((7 - row) * 8)) & 0xFF) as u8;
    }
    out
}

#[inline]
fn hud_8x8_glyph(ch: char) -> u64 {
    match ch {
        '0' => 0x3C66_6E76_6666_3C00,
        '1' => 0x1838_1818_1818_7E00,
        '2' => 0x3C66_061C_3066_7E00,
        '3' => 0x3C66_061C_0666_3C00,
        '4' => 0x0C1C_3C6C_7E0C_0C00,
        '5' => 0x7E60_7C06_0666_3C00,
        '6' => 0x3C60_7C66_6666_3C00,
        '7' => 0x7E66_060C_1818_1800,
        '8' => 0x3C66_663C_6666_3C00,
        '9' => 0x3C66_663E_060C_3800,
        'A' => 0x183C_6666_7E66_6600,
        'B' => 0x7C66_667C_6666_7C00,
        'C' => 0x3C66_6060_6066_3C00,
        'D' => 0x786C_6666_666C_7800,
        'E' => 0x7E60_6078_6060_7E00,
        'F' => 0x7E60_6078_6060_6000,
        'G' => 0x3C66_606E_6666_3E00,
        'H' => 0x6666_667E_6666_6600,
        'I' => 0x3C18_1818_1818_3C00,
        'J' => 0x1E0C_0C0C_6C6C_3800,
        'K' => 0x666C_7870_786C_6600,
        'L' => 0x6060_6060_6060_7E00,
        'M' => 0x6377_7F6B_6363_6300,
        'N' => 0x6676_7E7E_6E66_6600,
        'O' => 0x3C66_6666_6666_3C00,
        'P' => 0x7C66_667C_6060_6000,
        'Q' => 0x3C66_6666_6A6C_3600,
        'R' => 0x7C66_667C_6C66_6600,
        'S' => 0x3C66_603C_0666_3C00,
        'T' => 0x7E18_1818_1818_1800,
        'U' => 0x6666_6666_6666_3C00,
        'V' => 0x6666_6666_663C_1800,
        'W' => 0x6363_636B_7F77_6300,
        'X' => 0x6666_3C18_3C66_6600,
        'Y' => 0x6666_663C_1818_1800,
        'Z' => 0x7E06_0C18_3060_7E00,
        '/' => 0x060C_1830_60C0_8000,
        '[' => 0x3C30_3030_3030_3C00,
        ']' => 0x3C0C_0C0C_0C0C_3C00,
        '←' | '<' => 0x0C18_307E_3018_0C00,
        '→' | '>' => 0x3018_0C7E_0C18_3000,
        _ => 0x003C_3C3C_3C3C_0000,
    }
}

/// Overlays a crisp, zero-allocation 16-bit arcade scene badge on the bottom row during preview.
pub fn draw_preview_hud(
    cells: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    effect: EffectKind,
    tour_mode: bool,
) {
    if cols < 16 || rows < 3 || cells.len() < cols * rows {
        return;
    }
    let roster: &[EffectKind] = &EffectKind::ALL;
    let tag = if tour_mode { "TOUR" } else { "SCENE" };
    let idx = roster
        .iter()
        .position(|&k| k == effect)
        .map_or(1, |i| i + 1);
    let total = roster.len();
    let mut prefix_buf = [' '; 16];
    let mut p_len = 1;
    for b in tag.bytes() {
        prefix_buf[p_len] = char::from(b);
        p_len += 1;
    }
    let d = |n: usize| char::from(b'0' + (n % 10) as u8);
    #[rustfmt::skip]
    let tail = [' ', d(idx / 10), d(idx), '/', d(total / 10), d(total), ':', ' '];
    prefix_buf[p_len..p_len + 8].copy_from_slice(&tail);
    p_len += 8;

    let mut name_buf = [' '; 24];
    let mut n_len = 0;
    for b in effect.name().bytes().take(22) {
        name_buf[n_len] = char::from(b.to_ascii_uppercase());
        n_len += 1;
    }
    n_len += 1;

    const HINT_STR: &str = "  [←/→: STEP | Q/ESC: EXIT] ";
    let base_len = p_len + n_len;
    let show_hint = cols >= base_len + 32;
    let total_chars = base_len + if show_hint { 28 } else { 0 };
    if total_chars + 2 > cols {
        return;
    }
    let start_x = (cols - (total_chars + 2)) / 2;
    let row_off = (rows - 1) * cols;
    let pill_bg = crate::raster::subcell_overlay::HUD_PILL_BG;
    let set_cell = |c: &mut [TerminalCell], x: usize, ch: char, fg: (u8, u8, u8), bold: bool| {
        c[row_off + x] = TerminalCell {
            ch,
            fg,
            bg: pill_bg,
            bold,
        };
    };
    let cap = |ch, bg| TerminalCell {
        ch,
        fg: pill_bg,
        bg,
        bold: false,
    };
    cells[row_off + start_x] = cap('▐', cells[row_off + start_x].bg);
    let mut cx = start_x + 1;
    for &ch in &prefix_buf[..p_len] {
        set_cell(cells, cx, ch, (110, 210, 255), true);
        cx += 1;
    }
    for &ch in &name_buf[..n_len] {
        set_cell(cells, cx, ch, (255, 225, 85), true);
        cx += 1;
    }
    if show_hint {
        for ch in HINT_STR.chars() {
            set_cell(cells, cx, ch, (175, 195, 225), false);
            cx += 1;
        }
    }
    if cx < cols {
        cells[row_off + cx] = cap('▌', cells[row_off + cx].bg);
    }
}
