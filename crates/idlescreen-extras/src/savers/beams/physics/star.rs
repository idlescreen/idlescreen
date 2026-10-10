//! Star rendering for the beams screensaver (hero polish).

use crate::beams::light::{LightContext, get_light_at};
use crate::beams::types::{Spotlight, Star};
use crate::runner::TerminalCell;

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_star(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    ctx: &LightContext,
    stars: &[Star],
    twinkle_stars_opt: u32,
    time_elapsed: f32,
    spotlights: &[Spotlight],
    current_angles: &[f32],
    spot_cots: &[(f32, f32, f32, f32, f32)],
    accent: (u8, u8, u8),
    is_secondary: bool,
    intro_fade: f32,
) {
    if twinkle_stars_opt != 1 {
        return;
    }

    // Top-3 flare selection by excitation — fixed-size insertion, no
    // per-frame Vec allocs (was: collect + sort + take + collect).
    const MAX_FLARES: usize = 3;
    let mut flare_idx = [0usize; MAX_FLARES];
    let mut flare_score = [f32::MIN; MAX_FLARES];
    let mut n_flares = 0usize;
    for (i, star) in stars.iter().enumerate() {
        if star.excitation <= 0.85 || star.layer != 1 {
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

    let fade = intro_fade.clamp(0.0, 1.0);

    for (i, star) in stars.iter().enumerate() {
        let sx = (star.x * cols as f32) as usize;
        let sy = (star.y * rows as f32) as usize;
        if sx >= cols || sy >= rows {
            continue;
        }

        let (lr, lg, lb, intensity) = get_light_at(
            sx as f32,
            sy as f32,
            ctx,
            spotlights,
            current_angles,
            spot_cots,
            accent,
            is_secondary,
        );
        let on_primary = ctx.primary.contains(sx, sy) && !is_secondary;

        // Far stars: slow dim twinkle; near: brighter + react to beams
        let twinkle_rate = if star.layer == 0 { 1.1 } else { 2.5 };
        let sparkle_base = f32::midpoint((time_elapsed * twinkle_rate + star.phase).sin(), 1.0);
        let layer_dim = if star.layer == 0 { 0.45 } else { 1.0 };
        let sparkle = (sparkle_base * layer_dim + star.excitation).min(2.0);
        let brightness = (sparkle_base * 0.3 * layer_dim + star.excitation * 0.7).min(1.0);
        let final_brightness = sparkle * 0.35 + intensity * 0.55 * layer_dim;

        let mut star_r = (90.0 + brightness * 80.0) as u8;
        let mut star_g = (100.0 + brightness * 85.0) as u8;
        let mut star_b = (120.0 + brightness * 90.0) as u8;

        if (intensity > 0.1 || star.excitation > 0.05) && on_primary && star.layer == 1 {
            let blend = (star.excitation * 0.55 + intensity * 0.45).min(1.0);
            star_r = (star_r as f32 * (1.0 - blend) + lr * blend).min(255.0) as u8;
            star_g = (star_g as f32 * (1.0 - blend) + lg * blend).min(255.0) as u8;
            star_b = (star_b as f32 * (1.0 - blend) + lb * blend).min(255.0) as u8;
        }

        star_r = (star_r as f32 * fade) as u8;
        star_g = (star_g as f32 * fade) as u8;
        star_b = (star_b as f32 * fade) as u8;

        let cell = &mut grid[sy * cols + sx];
        cell.ch = if star.layer == 0 {
            star.ch
        } else if final_brightness > 0.85 {
            '✹'
        } else if final_brightness > 0.55 {
            '✦'
        } else {
            star.ch
        };
        cell.fg = (star_r, star_g, star_b);
        cell.bold = final_brightness > 0.65 || star.excitation > 0.35;

        // Rare, short flares (only top excited near stars)
        let is_excited = allowed_flares.contains(&i) && on_primary && star.excitation > 0.9;
        if is_excited {
            let flare_intensity = ((star.excitation - 0.85) / 0.6 + 0.35).min(1.2);
            let h_len = 8;
            for dx in 1..h_len {
                let alpha = (90.0f32 * flare_intensity).max(20.0f32) as u8;
                let fade_a = alpha.saturating_sub((dx * (100 / h_len)) as u8);
                if fade_a > 12 {
                    for &x in &[sx.saturating_add(dx), sx.saturating_sub(dx)] {
                        if x < cols {
                            let cell = &mut grid[sy * cols + x];
                            if cell.ch == ' ' || cell.ch == '\u{2500}' {
                                cell.ch = '\u{2500}';
                                cell.fg = (
                                    ((fade_a as f32 * 0.35 + lr * 0.55) * fade).min(255.0) as u8,
                                    ((fade_a as f32 * 0.3 + lg * 0.55) * fade).min(255.0) as u8,
                                    ((fade_a.saturating_add(30) as f32 * 0.35 + lb * 0.55) * fade)
                                        .min(255.0) as u8,
                                );
                            }
                        }
                    }
                }
            }
            let v_len = 3;
            for dy in 1..v_len {
                let alpha = (70.0f32 * flare_intensity).max(15.0f32) as u8;
                let fade_a = alpha.saturating_sub((dy * (70 / v_len)) as u8);
                if fade_a > 12 {
                    for &y in &[sy.saturating_add(dy), sy.saturating_sub(dy)] {
                        if y < rows {
                            let cell = &mut grid[y * cols + sx];
                            if cell.ch == ' ' || cell.ch == '\u{2502}' {
                                cell.ch = '\u{2502}';
                                cell.fg = (
                                    ((fade_a as f32 * 0.35 + lr * 0.55) * fade).min(255.0) as u8,
                                    ((fade_a as f32 * 0.3 + lg * 0.55) * fade).min(255.0) as u8,
                                    ((fade_a.saturating_add(25) as f32 * 0.35 + lb * 0.55) * fade)
                                        .min(255.0) as u8,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
