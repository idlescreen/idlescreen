// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Anamorphic horizontal lens flares, neon overdrive chroma boost, soft outer-glow halos,
//! and system-theme-adaptive bevel/starfield palettes ("So things pop").

use crate::ascii::sys_info::{ThemeColorScheme, detect_system_theme};
use crate::cell::TerminalCell;

/// Computes a theme-influenced 4-stop vertical chrome bevel gradient from an accent RGB.
#[inline]
pub fn theme_bevel_band(band: usize, accent: (u8, u8, u8)) -> (u8, u8, u8) {
    let (r, g, b) = (
        u16::from(accent.0),
        u16::from(accent.1),
        u16::from(accent.2),
    );
    match band {
        0 | 1 => (
            ((r * 2 + 510) / 3).min(255) as u8,
            ((g * 2 + 510) / 3).min(255) as u8,
            ((b * 2 + 510) / 3).min(255) as u8,
        ),
        2..=4 => (
            ((r * 3 + 255) / 4).min(255) as u8,
            ((g * 3 + 255) / 4).min(255) as u8,
            ((b * 3 + 255) / 4).min(255) as u8,
        ),
        5..=7 => (accent.0, accent.1, accent.2),
        _ => (
            ((r * 7) / 10).max(32) as u8,
            ((g * 7) / 10).max(48) as u8,
            ((b * 8) / 10).max(96) as u8,
        ),
    }
}

/// Applies neon overdrive chroma boost to make character colors pop with vivid saturation
/// and bright specular peaks without washing out hue separation.
#[inline]
pub fn neon_overdrive_rgb(c: (u8, u8, u8)) -> (u8, u8, u8) {
    let max_c = c.0.max(c.1).max(c.2);
    if max_c == 0 {
        let accent = detect_system_theme().accent_rgb;
        return accent;
    }
    let min_c = c.0.min(c.1).min(c.2);
    let chroma = max_c.saturating_sub(min_c);
    let floor = (min_c / 3).saturating_add(min_c / 4);
    let denom = u16::from(max_c.saturating_sub(floor)).max(1);
    let target_max = u16::from(max_c.max(175))
        .saturating_add(u16::from(chroma) / 4)
        .min(255);
    let scale = (target_max * 256) / denom;
    let chan = |v: u8| -> u8 {
        let spread = (u16::from(v.saturating_sub(floor)) * scale) >> 8;
        spread.min(255) as u8
    };
    (chan(c.0), chan(c.1), chan(c.2))
}

/// Renders anamorphic horizontal lens-flare streaks and soft neon outer-glow halos
/// originating from specular gleam hotspots (`dist == 0`) on banner glyphs.
pub fn apply_anamorphic_lens_flare(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    gleam_pos: usize,
    bg: (u8, u8, u8),
) {
    let total = cols.saturating_mul(rows).min(grid.len());
    if total == 0 || cols < 8 || rows == 0 {
        return;
    }
    let theme = detect_system_theme();
    let flare_core = (230, 252, 255);
    let flare_mid = blend_rgb((95, 225, 255), theme.accent_rgb, 110);
    let flare_tail = blend_rgb((42, 110, 210), theme.accent_rgb, 128);
    let halo_rgb = match theme.scheme {
        ThemeColorScheme::Dark => blend_rgb((28, 62, 132), theme.accent_rgb, 96),
        ThemeColorScheme::Light => blend_rgb((48, 92, 168), theme.accent_rgb, 120),
    };

    for y in 0..rows {
        let row_off = y * cols;
        if gleam_pos < y * 2 {
            continue;
        }
        let hot_x = gleam_pos - y * 2;
        if hot_x >= cols {
            continue;
        }
        let hot_idx = row_off + hot_x;
        if hot_idx >= total {
            continue;
        }
        let ch = grid[hot_idx].ch;
        if ch != '█' && ch != '▀' && ch != '▄' {
            continue;
        }

        // Anamorphic horizontal streak (left and right wings up to ±7 cells)
        for dx in 1..=7usize {
            let streak_ch = if dx <= 2 {
                '▓'
            } else if dx <= 4 {
                '▒'
            } else {
                '░'
            };
            let streak_fg = if dx <= 2 {
                flare_core
            } else if dx <= 4 {
                flare_mid
            } else {
                flare_tail
            };
            for target_x in [hot_x.checked_sub(dx), hot_x.checked_add(dx)]
                .into_iter()
                .flatten()
            {
                if target_x < cols {
                    let idx = row_off + target_x;
                    if idx < total && grid[idx].ch == ' ' {
                        grid[idx] = TerminalCell {
                            ch: streak_ch,
                            fg: streak_fg,
                            bg,
                            bold: dx <= 2,
                        };
                    }
                }
            }
        }

        // Soft vertical bloom / neon halo above and below the hotspot
        for dy in [-1isize, 1isize] {
            let ny = y as isize + dy;
            if ny >= 0 && (ny as usize) < rows {
                let n_idx = (ny as usize) * cols + hot_x;
                if n_idx < total && grid[n_idx].ch == ' ' {
                    grid[n_idx] = TerminalCell {
                        ch: '░',
                        fg: halo_rgb,
                        bg,
                        bold: false,
                    };
                }
            }
        }
    }
}

/// Blends two RGB colors (`weight_b` in `0..=256`).
#[inline]
pub fn blend_rgb(a: (u8, u8, u8), b: (u8, u8, u8), weight_b: u16) -> (u8, u8, u8) {
    let wb = weight_b.min(256);
    let wa = 256 - wb;
    (
        ((u16::from(a.0) * wa + u16::from(b.0) * wb) >> 8) as u8,
        ((u16::from(a.1) * wa + u16::from(b.1) * wb) >> 8) as u8,
        ((u16::from(a.2) * wa + u16::from(b.2) * wb) >> 8) as u8,
    )
}
