// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 3-layer 16-bit parallax starfield backdrop with per-scene randomized star
//! coordinates, 8 arcade color palettes, and varied parallax drift vectors.

use crate::cell::TerminalCell;

#[derive(Clone, Copy)]
struct StarfieldTheme {
    l1_primary: (u8, u8, u8),
    l1_accent: (u8, u8, u8),
    l2_primary: (u8, u8, u8),
    l2_secondary: (u8, u8, u8),
    l3_dust: (u8, u8, u8),
    density: (usize, usize, usize),
}

const THEMES: [StarfieldTheme; 8] = [
    // 0: Deep Sapphire & SNES Gold
    StarfieldTheme {
        l1_primary: (95, 220, 255),
        l1_accent: (255, 200, 60),
        l2_primary: (215, 85, 210),
        l2_secondary: (130, 95, 255),
        l3_dust: (40, 78, 156),
        density: (72, 96, 130),
    },
    // 1: Synthwave Violet & Neon Magenta
    StarfieldTheme {
        l1_primary: (255, 110, 220),
        l1_accent: (120, 240, 255),
        l2_primary: (185, 75, 245),
        l2_secondary: (240, 140, 90),
        l3_dust: (72, 38, 128),
        density: (68, 92, 136),
    },
    // 2: Emerald Aurora & Cyber Teal
    StarfieldTheme {
        l1_primary: (110, 255, 175),
        l1_accent: (220, 255, 120),
        l2_primary: (55, 205, 195),
        l2_secondary: (70, 165, 235),
        l3_dust: (28, 96, 88),
        density: (70, 94, 126),
    },
    // 3: Crimson Nebula & Solar Amber
    StarfieldTheme {
        l1_primary: (255, 215, 90),
        l1_accent: (255, 130, 95),
        l2_primary: (225, 70, 110),
        l2_secondary: (175, 85, 210),
        l3_dust: (112, 36, 58),
        density: (64, 90, 122),
    },
    // 4: Cyber Cyan & Glacial Ice
    StarfieldTheme {
        l1_primary: (210, 250, 255),
        l1_accent: (90, 230, 255),
        l2_primary: (75, 165, 255),
        l2_secondary: (115, 130, 245),
        l3_dust: (32, 74, 136),
        density: (76, 100, 138),
    },
    // 5: Golden Arcade & Royal Indigo
    StarfieldTheme {
        l1_primary: (255, 225, 95),
        l1_accent: (255, 165, 70),
        l2_primary: (120, 115, 255),
        l2_secondary: (90, 195, 245),
        l3_dust: (52, 48, 132),
        density: (70, 96, 128),
    },
    // 6: Jade Matrix & Phosphor Green
    StarfieldTheme {
        l1_primary: (150, 255, 140),
        l1_accent: (90, 245, 210),
        l2_primary: (65, 195, 115),
        l2_secondary: (110, 180, 90),
        l3_dust: (26, 88, 48),
        density: (66, 88, 120),
    },
    // 7: Twilight Void & Starlight Silver
    StarfieldTheme {
        l1_primary: (235, 240, 255),
        l1_accent: (255, 205, 125),
        l2_primary: (145, 135, 215),
        l2_secondary: (95, 165, 220),
        l3_dust: (48, 54, 108),
        density: (60, 84, 116),
    },
];

/// Computes the 3-layer parallax coordinate offsets `((dx1, dy1), (dx2, dy2), (dx3, dy3))`
/// for the given `drift_mode` (`0..8`) and frame `tick`.
#[inline]
fn layer_offsets(
    drift_mode: usize,
    tick: usize,
) -> ((usize, usize), (usize, usize), (usize, usize)) {
    let (fast, med, slow) = (tick.wrapping_mul(2), tick / 2, tick / 3);
    match drift_mode & 7 {
        0 => ((fast, 0), (0, med), (slow, 0)),
        1 => (
            (0, usize::MAX.wrapping_sub(fast)),
            (med, 0),
            (0, usize::MAX.wrapping_sub(slow)),
        ),
        2 => ((0, fast), (usize::MAX.wrapping_sub(med), 0), (0, slow)),
        3 => (
            (fast, usize::MAX.wrapping_sub(med)),
            (med, usize::MAX.wrapping_sub(slow)),
            (slow, 0),
        ),
        4 => (
            (usize::MAX.wrapping_sub(fast), 0),
            (0, usize::MAX.wrapping_sub(med)),
            (usize::MAX.wrapping_sub(slow), 0),
        ),
        5 => ((fast, med), (med, slow), (slow, tick / 5)),
        6 => ((fast, 0), (med, med), (0, slow)),
        _ => ((tick, 0), (0, slow), (tick / 4, tick / 6)),
    }
}

#[inline]
fn spatial_hash(x: usize, y: usize, salt: usize, mul_x: usize, mul_y: usize) -> usize {
    let mut h = x.wrapping_add(salt).wrapping_mul(mul_x)
        ^ y.wrapping_add(salt.rotate_left(13)).wrapping_mul(mul_y)
        ^ salt.wrapping_mul(0x9E37_79B9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h
}

/// Derives a per-scene starfield backdrop seed that cycles through all 8 themes and
/// 8 drift vectors on consecutive scenes while randomizing star coordinates.
#[inline]
pub fn compute_backdrop_seed(effect_idx: u64, seed: u64) -> u64 {
    ((seed ^ effect_idx.wrapping_mul(0x9E37_79B9_7F4A_7C15)) & !0x3F)
        | ((effect_idx.wrapping_mul(3) & 7) << 3)
        | (effect_idx & 7)
}

/// Paints the 3-layer 16-bit parallax starfield into empty (`' '`) cells of `grid`.
/// Uses `backdrop_seed` to vary the star positions, drift direction, and color theme per scene.
pub fn paint_starfield_backdrop(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    frame_tick: u64,
    backdrop_seed: u64,
    bg: (u8, u8, u8),
) {
    let total = cols.saturating_mul(rows).min(grid.len());
    if total == 0 || cols == 0 || rows == 0 {
        return;
    }

    let seed = backdrop_seed as usize;
    let theme_idx = seed & 7;
    let drift_mode = (seed >> 3) & 7;
    let base_theme = &THEMES[theme_idx];
    let sys_accent = crate::ascii::sys_info::detect_system_theme().accent_rgb;
    let theme = if sys_accent == (81, 162, 218) {
        *base_theme
    } else {
        use super::blend_rgb;
        StarfieldTheme {
            l1_primary: blend_rgb(base_theme.l1_primary, sys_accent, 96),
            l1_accent: base_theme.l1_accent,
            l2_primary: blend_rgb(base_theme.l2_primary, sys_accent, 80),
            l2_secondary: base_theme.l2_secondary,
            l3_dust: blend_rgb(base_theme.l3_dust, sys_accent, 64),
            density: base_theme.density,
        }
    };
    let (d1, d2, d3) = theme.density;

    let tick = frame_tick as usize;
    let ((dx1, dy1), (dx2, dy2), (dx3, dy3)) = layer_offsets(drift_mode, tick);

    let salt1 = seed.wrapping_mul(0x9E37_79B9).wrapping_add(0x1357_9BDF);
    let salt2 = seed.wrapping_mul(0x85EB_CA6B).wrapping_add(0x2468_ACE1);
    let salt3 = seed.wrapping_mul(0xC2B2_AE35).wrapping_add(0x369C_F258);
    let twinkle_tick = tick / 12;

    for y in 0..rows {
        let row_off = y * cols;
        let y1 = y.wrapping_add(dy1);
        let y2 = y.wrapping_add(dy2);
        let y3 = y.wrapping_add(dy3);

        for x in 0..cols {
            let idx = row_off + x;
            if idx >= total {
                break;
            }
            if grid[idx].ch != ' ' {
                continue;
            }

            let h1_full =
                spatial_hash(x.wrapping_add(dx1), y1, salt1, 2_654_435_761, 2_246_822_519);
            let h1 = h1_full & 0xFFFF;
            if h1 < d1 {
                let is_accent = (h1_full >> 16).is_multiple_of(4);
                let phase = ((h1_full >> 18).wrapping_add(twinkle_tick)) & 1;
                grid[idx] = TerminalCell {
                    ch: if phase == 0 { '▘' } else { '▝' },
                    fg: if is_accent {
                        theme.l1_accent
                    } else {
                        theme.l1_primary
                    },
                    bg,
                    bold: true,
                };
                continue;
            }

            let h2_full = spatial_hash(x.wrapping_add(dx2), y2, salt2, 3_266_489_917, 668_265_263);
            let h2 = h2_full & 0xFFFF;
            if h2 < d2 {
                let phase = ((h2_full >> 16).wrapping_add(twinkle_tick >> 1)) & 1;
                grid[idx] = TerminalCell {
                    ch: if phase == 0 { '▗' } else { '▖' },
                    fg: if (h2_full & 0x20000) == 0 {
                        theme.l2_primary
                    } else {
                        theme.l2_secondary
                    },
                    bg,
                    bold: false,
                };
                continue;
            }

            let h3_full = spatial_hash(x.wrapping_add(dx3), y3, salt3, 374_761_393, 1_103_515_245);
            let h3 = h3_full & 0xFFFF;
            if h3 < d3 {
                grid[idx] = TerminalCell {
                    ch: '░',
                    fg: theme.l3_dust,
                    bg,
                    bold: false,
                };
            }
        }
    }
}
