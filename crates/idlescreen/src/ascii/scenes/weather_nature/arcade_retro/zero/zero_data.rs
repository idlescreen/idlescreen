// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Authentic SNES Mega Man X3 Zero sprite atlas parser, 36-color palette,
//! Z-Buster / Z-Saber / Maverick explosion palette remapping, and sub-cell blitter.

use crate::ascii::scenes::canvas_ops::hex_color;
use std::sync::OnceLock;
use ttfx::utils::graphics::Color;

const RAW_SPRITES: &str = include_str!("zero_sprites.txt");

#[rustfmt::skip]
pub const ZERO_PALETTE_HEX: [&str; 36] = [
    "#981018", "#D82028", "#F83840", "#FF7878", "#F8F8F8", "#2088F8",
    "#68C8FF", "#A8B8C8", "#884818", "#C07050", "#F8B888", "#2C3246",
    "#64708C", "#B87810", "#F8C820", "#FFF888", "#109838", "#00D848",
    "#40FF78", "#A8FFC8", "#E8FFF0", "#FFFFFF", "#F86820", "#F8A828",
    "#FFF078", "#FF2868", "#7818A8", "#B830F8", "#E868FF", "#FF38A8",
    "#586880", "#90A8C8", "#D0E8FF", "#FF1838", "#FF7088", "#FFD8E0",
];

pub struct SpriteFrame {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

pub struct ZeroAtlas {
    pub beam: SpriteFrame,
    pub splash: SpriteFrame,
    pub mat1: SpriteFrame,
    pub mat2: SpriteFrame,
    pub mat3: SpriteFrame,
    pub idle1: SpriteFrame,
    pub idle2: SpriteFrame,
    pub buster1: SpriteFrame,
    pub buster2: SpriteFrame,
    pub saber_prep: SpriteFrame,
    pub saber_swing1: SpriteFrame,
    pub saber_swing2: SpriteFrame,
    pub saber_pose: SpriteFrame,
    pub shot1: SpriteFrame,
    pub shot2: SpriteFrame,
    pub wave1: SpriteFrame,
    pub wave2: SpriteFrame,
    pub slash_exp1: SpriteFrame,
    pub slash_exp2: SpriteFrame,
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

pub fn atlas() -> &'static ZeroAtlas {
    static ATLAS: OnceLock<ZeroAtlas> = OnceLock::new();
    ATLAS.get_or_init(|| ZeroAtlas {
        beam: parse_frame("beam"),
        splash: parse_frame("splash"),
        mat1: parse_frame("mat1"),
        mat2: parse_frame("mat2"),
        mat3: parse_frame("mat3"),
        idle1: parse_frame("idle1"),
        idle2: parse_frame("idle2"),
        buster1: parse_frame("buster1"),
        buster2: parse_frame("buster2"),
        saber_prep: parse_frame("saber_prep"),
        saber_swing1: parse_frame("saber_swing1"),
        saber_swing2: parse_frame("saber_swing2"),
        saber_pose: parse_frame("saber_pose"),
        shot1: parse_frame("shot1"),
        shot2: parse_frame("shot2"),
        wave1: parse_frame("wave1"),
        wave2: parse_frame("wave2"),
        slash_exp1: parse_frame("slash_exp1"),
        slash_exp2: parse_frame("slash_exp2"),
    })
}

pub fn build_zero_colors() -> Vec<Color> {
    ZERO_PALETTE_HEX.iter().map(|hex| hex_color(hex)).collect()
}

pub fn zero_palette_rgb() -> &'static [(u8, u8, u8); 36] {
    static RGB: OnceLock<[(u8, u8, u8); 36]> = OnceLock::new();
    RGB.get_or_init(|| {
        let mut out = [(0u8, 0u8, 0u8); 36];
        for (i, hex) in ZERO_PALETTE_HEX.iter().enumerate() {
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
        // Mode 1: Crimson/Gold Z-Buster Flash on Zero's armor
        (1, 1) => 23, (1, 2) => 24, (1, 3) => 25, (1, 5) => 22, (1, 13) => 34,
        // Mode 2: Neon-Emerald Z-Saber Overdrive Aura
        (2, 1) => 17, (2, 2) => 18, (2, 3) => 19, (2, 5) => 20, (2, 13) => 18,
        // Mode 3: Violet/Magenta Maverick Plasma Explosion & Cross-Slash variant
        (3, 17) => 27, (3, 18) => 28, (3, 19) => 29, (3, 20) => 30,
        (3, 23) => 28, (3, 24) => 29, (3, 25) => 30, (3, 26) => 27,
        _ => idx,
    }
}

#[inline]
#[rustfmt::skip]
fn pixel_priority(raw: u8) -> u8 {
    match raw {
        0 => 0, 12 => 1, 13 => 2, 4 | 5 | 8 => 4, 1 | 9 => 5,
        2 | 3 | 10 | 11 => 6, 14..=16 => 8, 6 | 7 | 17..=22 => 10,
        _ => 7,
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
    let center =
        sprite.pixels[usize::midpoint(sy0, sy1) * sprite.width + usize::midpoint(sx0, sx1)];
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
