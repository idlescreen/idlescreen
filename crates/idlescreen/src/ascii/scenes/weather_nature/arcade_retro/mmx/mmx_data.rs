// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Authentic SNES Mega Man X (X1) sprite atlas parser, 36-color palette,
//! Level-3 charge/spiral palette remapping, and half-block pixel blitter.

use crate::ascii::scenes::canvas_ops::hex_color;
use std::sync::OnceLock;
use ttfx::utils::graphics::Color;

const RAW_SPRITES: &str = include_str!("mmx_sprites.txt");

#[rustfmt::skip]
pub const MMX_PALETTE_HEX: [&str; 36] = [
    "#0040F0", "#0080F8", "#50A0F0", "#78D8F0", "#F0F0F0", "#F04010",
    "#1858B0", "#808080", "#804020", "#B86048", "#F8B080", "#243460",
    "#203080", "#E8E040", "#E0E0E0", "#688090", "#00A0F8", "#0060F8",
    "#C0F0F8", "#30E8F8", "#00C8F8", "#F8F8F8", "#F86038", "#F09060",
    "#F8C038", "#F8E8B8", "#C0C0C0", "#A0A0A0", "#888888", "#D82818",
    "#C83870", "#F850A8", "#F8A0D8", "#F8C0F0", "#F8A020", "#FFF090",
];

pub struct SpriteFrame {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

pub struct MmxAtlas {
    pub beam: SpriteFrame,
    pub splash: SpriteFrame,
    pub mat1: SpriteFrame,
    pub mat2: SpriteFrame,
    pub idle: SpriteFrame,
    pub aim: SpriteFrame,
    pub fire: SpriteFrame,
    pub spiral1: SpriteFrame,
    pub spiral2: SpriteFrame,
    pub orb: SpriteFrame,
    pub exp1: SpriteFrame,
    pub exp2: SpriteFrame,
}

#[inline]
fn decode_pixel_char(b: u8) -> u8 {
    match b {
        b'A'..=b'Z' => b - b'A' + 1,
        b'a'..=b'j' => b - b'a' + 27,
        _ => 0,
    }
}

fn parse_frame(name: &str) -> SpriteFrame {
    let header_prefix = format!("@{name} ");
    let mut lines = RAW_SPRITES.lines();
    let mut width = 0usize;
    let mut height = 0usize;
    for line in lines.by_ref() {
        if let Some(rest) = line.strip_prefix(&header_prefix) {
            let mut parts = rest.split_whitespace();
            width = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            height = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            break;
        }
    }
    let mut pixels = Vec::with_capacity(width * height);
    for line in lines.take(height) {
        let bytes = line.as_bytes();
        for x in 0..width {
            let b = bytes.get(x).copied().unwrap_or(b'.');
            pixels.push(decode_pixel_char(b));
        }
    }
    pixels.resize(width * height, 0);
    SpriteFrame {
        width,
        height,
        pixels,
    }
}

pub fn atlas() -> &'static MmxAtlas {
    static ATLAS: OnceLock<MmxAtlas> = OnceLock::new();
    ATLAS.get_or_init(|| MmxAtlas {
        beam: parse_frame("beam"),
        splash: parse_frame("splash"),
        mat1: parse_frame("mat1"),
        mat2: parse_frame("mat2"),
        idle: parse_frame("idle"),
        aim: parse_frame("aim"),
        fire: parse_frame("fire"),
        spiral1: parse_frame("spiral1"),
        spiral2: parse_frame("spiral2"),
        orb: parse_frame("orb"),
        exp1: parse_frame("exp1"),
        exp2: parse_frame("exp2"),
    })
}

pub fn build_mmx_colors() -> Vec<Color> {
    MMX_PALETTE_HEX.iter().map(|hex| hex_color(hex)).collect()
}

pub fn mmx_palette_rgb() -> &'static [(u8, u8, u8); 36] {
    static RGB: OnceLock<[(u8, u8, u8); 36]> = OnceLock::new();
    RGB.get_or_init(|| {
        let mut out = [(0u8, 0u8, 0u8); 36];
        for (i, hex) in MMX_PALETTE_HEX.iter().enumerate() {
            let s = hex.trim_start_matches('#');
            if s.len() >= 6 {
                let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
                out[i] = (r, g, b);
            }
        }
        out
    })
}

#[inline]
#[rustfmt::skip]
pub fn remap_palette(idx: u8, mode: u8) -> u8 {
    match (mode, idx) {
        // Mode 1: Level-2 Cyan/Gold Charge Flash on X's armor
        (1, 1) => 20, (1, 2) => 19, (1, 3) => 36, (1, 4) => 22, (1, 13) => 35,
        // Mode 2: Level-3 Pink/Gold Max-Charge Aura on X's armor
        (2, 1) => 31, (2, 2) => 32, (2, 3) => 35, (2, 4) => 36, (2, 7) => 33, (2, 13) => 25,
        // Mode 3: SNES Level-3 Pink/Magenta/Gold Spiral Crush Shot
        (3, 17) => 32, (3, 18) => 31, (3, 19) => 34, (3, 20) => 33, (3, 21) => 35, (3, 22) => 36,
        _ => idx,
    }
}

#[inline]
#[rustfmt::skip]
fn pixel_priority(raw: u8) -> u8 {
    match raw {
        0 => 0, 12 => 1, 13 => 2, 7 | 8 | 16 => 3, 1 | 15 => 4,
        2 => 5, 3 => 6, 4 | 9 | 10 => 7, 5 | 14 => 8, 11 => 9, 6 => 10,
        _ => raw,
    }
}

#[inline]
pub fn sample_subcell_halfblock(px_buf: &[u8], sub_w: usize, cx: usize, hy: usize) -> u8 {
    let (sx0, sy0) = (cx * 4, hy * 4);
    let mut scores = [0u8; 37];
    let (mut best_raw, mut best_score) = (0u8, 0u8);
    for dy in 0..4 {
        let row = (sy0 + dy) * sub_w;
        for dx in 0..4 {
            let raw = px_buf.get(row + sx0 + dx).copied().unwrap_or(0).min(36);
            if raw != 0 {
                let w = pixel_priority(raw)
                    + u8::from((1..=2).contains(&dx) && (1..=2).contains(&dy)) * 2;
                scores[raw as usize] = scores[raw as usize].saturating_add(w);
                if scores[raw as usize] >= best_score {
                    (best_raw, best_score) = (raw, scores[raw as usize]);
                }
            }
        }
    }
    best_raw
}

#[inline]
fn sample_window(sprite: &SpriteFrame, sx0: usize, sx1: usize, sy0: usize, sy1: usize) -> u8 {
    let center = sprite.pixels[((sy0 + sy1) / 2) * sprite.width + ((sx0 + sx1) / 2)];
    let (mut best_raw, mut best_prio) = (center, pixel_priority(center));
    let (mut non_zero, mut total) = (0usize, 0usize);
    for sy in sy0..sy1 {
        let row = sy * sprite.width;
        for sx in sx0..sx1 {
            total += 1;
            let raw = sprite.pixels[row + sx];
            if raw != 0 {
                non_zero += 1;
                let prio = pixel_priority(raw);
                if prio > best_prio {
                    (best_raw, best_prio) = (raw, prio);
                }
            }
        }
    }
    if center != 0 || non_zero * 2 >= total || best_prio >= 9 {
        best_raw
    } else {
        0
    }
}

#[allow(clippy::too_many_arguments)]
pub fn blit_sprite(
    px_buf: &mut [u8],
    buf_w: usize,
    buf_h: usize,
    sprite: &SpriteFrame,
    dst_x: isize,
    dst_y: isize,
    mode: u8,
    scale_den: usize,
) {
    if sprite.width == 0 || sprite.height == 0 || buf_w == 0 || buf_h == 0 {
        return;
    }
    let den = scale_den.max(10);
    let out_w = ((sprite.width * 40) / den).max(1);
    let out_h = ((sprite.height * 40) / den).max(1);
    let direct = out_w >= sprite.width && out_h >= sprite.height;
    for dy in 0..out_h {
        let py = dst_y + dy as isize;
        if py < 0 || py >= buf_h as isize {
            continue;
        }
        let sy0 = ((dy * sprite.height) / out_h).min(sprite.height - 1);
        let sy1 = (((dy + 1) * sprite.height) / out_h)
            .max(sy0 + 1)
            .min(sprite.height);
        let row_off = (py as usize) * buf_w;
        for dx in 0..out_w {
            let px = dst_x + dx as isize;
            if px < 0 || px >= buf_w as isize {
                continue;
            }
            let sx0 = ((dx * sprite.width) / out_w).min(sprite.width - 1);
            let raw = if direct {
                sprite.pixels[sy0 * sprite.width + sx0]
            } else {
                let sx1 = (((dx + 1) * sprite.width) / out_w)
                    .max(sx0 + 1)
                    .min(sprite.width);
                sample_window(sprite, sx0, sx1, sy0, sy1)
            };
            if raw != 0 {
                px_buf[row_off + (px as usize)] = remap_palette(raw, mode);
            }
        }
    }
}
