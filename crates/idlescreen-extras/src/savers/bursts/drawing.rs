//! Helper drawing functions for the bursts screensaver.

use super::physics::{
    blend_horizontal_flare_color, blend_vertical_flare_color, calculate_logo_illumination,
    calculate_star_color_and_sparkle, calculate_star_illumination, calculate_window_glow,
    calculate_window_illumination,
};
use super::types::{ActiveExplosion, Star};
use crate::runner::TerminalCell;

pub fn draw_stars(
    stars: &[Star],
    skyline: &[usize],
    active_explosions: &[ActiveExplosion],
    time_elapsed: f32,
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
) {
    // Top-4 flare selection by excitation — fixed-size insertion, no
    // per-frame Vec allocs (was: collect + sort + take + collect).
    const MAX_FLARES: usize = 4;
    let mut flare_idx = [0usize; MAX_FLARES];
    let mut flare_score = [f32::MIN; MAX_FLARES];
    let mut n_flares = 0usize;
    for (i, star) in stars.iter().enumerate() {
        if star.excitation <= 0.8 {
            continue;
        }
        if n_flares == MAX_FLARES && star.excitation <= flare_score[MAX_FLARES - 1] {
            continue;
        }
        let mut pos = n_flares.min(MAX_FLARES - 1);
        while pos > 0 && star.excitation > flare_score[pos - 1] {
            flare_score[pos] = flare_score[pos - 1];
            flare_idx[pos] = flare_idx[pos - 1];
            pos -= 1;
        }
        flare_score[pos] = star.excitation;
        flare_idx[pos] = i;
        n_flares = (n_flares + 1).min(MAX_FLARES);
    }
    let allowed_flares = &flare_idx[..n_flares];

    for (i, star) in stars.iter().enumerate() {
        let sx = (star.x * cols as f32) as usize;
        let sy = (star.y * rows as f32) as usize;
        if sx < cols && sy < rows {
            let height_from_bottom = rows.saturating_sub(1).saturating_sub(sy);
            if height_from_bottom >= skyline[sx] {
                let (best_intensity, lit_color) =
                    calculate_star_illumination(sx as f32, sy as f32, active_explosions);

                let (color, sparkle) =
                    calculate_star_color_and_sparkle(time_elapsed, star, lit_color, best_intensity);

                let final_brightness = sparkle * 0.4 + best_intensity * 0.6;

                let ch = if final_brightness > 0.8 {
                    '✹'
                } else if final_brightness > 0.5 {
                    '✦'
                } else {
                    star.ch
                };

                grid[sy * cols + sx] = TerminalCell {
                    ch,
                    fg: color,
                    bg: (0, 0, 0),
                    bold: final_brightness > 0.6 || star.excitation > 0.3,
                };

                let is_excited = allowed_flares.contains(&i);
                if is_excited {
                    let flare_intensity = ((star.excitation - 0.8) / 0.7 + 0.5).min(1.5);
                    let flare_color = star.excited_color;

                    let h_len = 12;
                    for dx in 1..h_len {
                        let alpha = (120.0 * flare_intensity).max(30.0) as u8;
                        let fade = alpha.saturating_sub((dx * (110 / h_len)) as u8);
                        if fade > 10 {
                            if sx + dx < cols {
                                let cell = &mut grid[sy * cols + (sx + dx)];
                                let h_test = rows.saturating_sub(1).saturating_sub(sy);
                                if h_test >= skyline[sx + dx] && (cell.ch == ' ' || cell.ch == '─')
                                {
                                    cell.ch = '─';
                                    cell.fg = blend_horizontal_flare_color(fade, flare_color);
                                }
                            }
                            if sx >= dx {
                                let cell = &mut grid[sy * cols + (sx - dx)];
                                let h_test = rows.saturating_sub(1).saturating_sub(sy);
                                if h_test >= skyline[sx - dx] && (cell.ch == ' ' || cell.ch == '─')
                                {
                                    cell.ch = '─';
                                    cell.fg = blend_horizontal_flare_color(fade, flare_color);
                                }
                            }
                        }
                    }

                    let v_len = 5;
                    for dy in 1..v_len {
                        let alpha = (90.0 * flare_intensity).max(20.0) as u8;
                        let fade = alpha.saturating_sub((dy * (80 / v_len)) as u8);
                        if fade > 10 {
                            if sy + dy < rows {
                                let cell = &mut grid[(sy + dy) * cols + sx];
                                let h_test = rows.saturating_sub(1).saturating_sub(sy + dy);
                                if h_test >= skyline[sx] && (cell.ch == ' ' || cell.ch == '│') {
                                    cell.ch = '│';
                                    cell.fg = blend_vertical_flare_color(fade, flare_color);
                                }
                            }
                            if sy >= dy {
                                let cell = &mut grid[(sy - dy) * cols + sx];
                                let h_test = rows.saturating_sub(1).saturating_sub(sy - dy);
                                if h_test >= skyline[sx] && (cell.ch == ' ' || cell.ch == '│') {
                                    cell.ch = '│';
                                    cell.fg = blend_vertical_flare_color(fade, flare_color);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn draw_logo(
    logo_text: &str,
    active_explosions: &[ActiveExplosion],
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    accent: (u8, u8, u8),
) {
    let default_color = (
        (accent.0 as f32 * 0.15) as u8,
        (accent.1 as f32 * 0.15) as u8,
        (accent.2 as f32 * 0.15) as u8,
    );
    let Some(logo) = crate::runner::place_centered_logo(cols, rows, logo_text, None) else {
        return;
    };

    for (r_offset, line) in logo.lines.iter().enumerate() {
        let gy = logo.y + r_offset;
        if gy >= rows {
            continue;
        }
        for (c_offset, ch) in line.chars().enumerate() {
            let gx = logo.x + c_offset;
            if gx >= cols {
                continue;
            }
            if ch != ' ' {
                let (best_intensity, lit_color) = calculate_logo_illumination(
                    gx as f32,
                    gy as f32,
                    active_explosions,
                    default_color,
                );

                grid[gy * cols + gx] = TerminalCell {
                    ch,
                    fg: lit_color,
                    bg: (0, 0, 0),
                    bold: best_intensity > 0.1,
                };
            }
        }
    }
}

pub fn draw_skyline(
    skyline: &[usize],
    skyline_windows: &[bool],
    active_explosions: &[ActiveExplosion],
    time_elapsed: f32,
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
) {
    for (c, &building_h) in skyline.iter().enumerate().take(cols) {
        for r in 0..building_h {
            let gy = rows.saturating_sub(1).saturating_sub(r);
            let idx = gy * cols + c;

            let is_lit_window = skyline_windows.get(idx).copied().unwrap_or(false);
            // Uneven per-window pulse so the city never looks flat.
            let twinkle = if is_lit_window {
                let phase = (c as f32 * 0.37 + r as f32 * 0.61 + time_elapsed * 1.7).sin();
                0.72 + 0.28 * ((phase * 0.5 + 0.5).powf(1.4))
            } else {
                1.0
            };
            let base_fg = if is_lit_window {
                (
                    (255.0 * twinkle) as u8,
                    (220.0 * twinkle) as u8,
                    (100.0 * twinkle) as u8,
                )
            } else {
                (0, 0, 0)
            };

            let (best_intensity, glow_color) =
                calculate_window_illumination(c as f32, gy as f32, active_explosions);

            let (ch, fg) = if best_intensity > 0.05 {
                calculate_window_glow(is_lit_window, base_fg, glow_color, best_intensity)
            } else {
                (if is_lit_window { '■' } else { ' ' }, base_fg)
            };

            grid[idx] = TerminalCell {
                ch,
                fg,
                bg: (15, 15, 22),
                bold: best_intensity > 0.2 || (is_lit_window && twinkle > 0.92),
            };
        }
    }
}
