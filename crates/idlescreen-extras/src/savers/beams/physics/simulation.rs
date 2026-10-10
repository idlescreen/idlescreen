//! Core calculations and helper functions for the beams screensaver.

use super::star::draw_star;
use crate::beams::light::{LightContext, draw_spotlight, get_light_at};
use crate::beams::types::{DustParticle, Spotlight, Star};
use crate::runner::TerminalCell;

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_dust(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    ctx: &LightContext,
    particles: &[DustParticle],
    spotlights: &[Spotlight],
    current_angles: &[f32],
    spot_cots: &[(f32, f32, f32, f32, f32)],
    accent: (u8, u8, u8),
    is_secondary: bool,
    intro_fade: f32,
) {
    let fade = intro_fade.clamp(0.0, 1.0);
    for p in particles {
        let px = (p.x * cols as f32) as usize;
        let py = (p.y * rows as f32) as usize;
        if px < cols && py < rows {
            let on_primary = ctx.primary.contains(px, py) && !is_secondary;
            let (lr, lg, lb, intensity) = if !on_primary && is_secondary {
                (60.0, 60.0, 60.0, 0.15)
            } else {
                get_light_at(
                    px as f32,
                    py as f32,
                    ctx,
                    spotlights,
                    current_angles,
                    spot_cots,
                    accent,
                    is_secondary,
                )
            };

            // Far plane: only dim motes; near plane: beam-reactive
            let layer_mul = if p.layer == 0 { 0.45 } else { 1.0 };
            let intensity = intensity * layer_mul;

            // Outside beam: far dust can still show faintly; near dust mostly hidden
            let use_baseline = p.layer == 0 && intensity <= 0.04 && on_primary;
            let draw_it = intensity > 0.04 || use_baseline || p.spark > 0.05;

            if draw_it {
                let sparking = p.spark > 0.05;
                let ch = if sparking {
                    if p.spark > 0.2 { '✧' } else { '*' }
                } else if use_baseline {
                    '·'
                } else if intensity > 0.65 {
                    '*'
                } else if intensity > 0.35 {
                    '+'
                } else if p.layer == 0 {
                    '·'
                } else {
                    '.'
                };

                let (p_r, p_g, p_b) = if sparking {
                    let s = p.spark.min(1.0);
                    (
                        (lr * (0.5 + s * 0.5) + 80.0 * s).min(255.0) as u8,
                        (lg * (0.5 + s * 0.5) + 60.0 * s).min(255.0) as u8,
                        (lb * (0.5 + s * 0.5) + 40.0 * s).min(255.0) as u8,
                    )
                } else if use_baseline || is_secondary {
                    (55, 55, 70)
                } else {
                    (
                        (100.0 * (1.0 - intensity) + lr * intensity).min(255.0) as u8,
                        (80.0 * (1.0 - intensity) + lg * intensity).min(255.0) as u8,
                        (160.0 * (1.0 - intensity) + lb * intensity).min(255.0) as u8,
                    )
                };

                // Apply intro fade to dust brightness
                let p_r = (p_r as f32 * fade) as u8;
                let p_g = (p_g as f32 * fade) as u8;
                let p_b = (p_b as f32 * fade) as u8;

                let cell = &mut grid[py * cols + px];
                let current_ch = cell.ch;
                if current_ch == ' ' || current_ch == '\u{2500}' || current_ch == '\u{2502}' {
                    cell.ch = ch;
                    cell.fg = (p_r, p_g, p_b);
                    cell.bold = sparking || intensity > 0.55;
                }
            }
        }
    }
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_impl(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    spotlights: &[Spotlight],
    stars: &[Star],
    particles: &[DustParticle],
    twinkle_stars_opt: u32,
    time_elapsed: f32,
    logo_text: &str,
    accent: (u8, u8, u8),
    intro_fade: f32,
) {
    // Beam 1's live accent is applied inside get_light_at — no clone needed.
    let is_secondary = crate::runner::is_secondary_monitor();

    let audio = idle_api::audio::query_audio_bands();
    let surge = (audio[0] * 1.4 + audio[1] * 0.6).clamp(0.0, 1.0);

    let mut current_angles = Vec::new();
    let mut spot_cots = Vec::new();
    for spot in spotlights {
        // Per-beam calm: only this cone eases amplitude, others keep sweeping.
        let amp = spot.angle_amplitude * (0.35 + 0.65 * spot.motion_blend);
        let angle = spot.angle_center + amp * (spot.phase + spot.phase_offset).sin();
        current_angles.push(angle);

        // Cot culling uses outer soft cone with audio surge modulation
        let spread = spot.spread * (1.0 + surge * 0.5);
        let outer = spread * 1.75;
        let a_min = angle - outer;
        let a_max = angle + outer;

        let cot_min = if a_min > 1e-4 {
            let (sin, cos) = a_min.sin_cos();
            cos / sin.max(1e-6)
        } else {
            0.0
        };

        let cot_max = if a_max < std::f32::consts::PI - 1e-4 {
            let (sin, cos) = a_max.sin_cos();
            cos / sin.max(1e-6)
        } else {
            0.0
        };

        spot_cots.push((a_min, a_max, cot_min, cot_max, spread));
    }

    let light_ctx = LightContext::new(cols, rows, spotlights);

    draw_spotlight(
        grid,
        cols,
        rows,
        &light_ctx,
        spotlights,
        &current_angles,
        &spot_cots,
        accent,
        is_secondary,
        intro_fade,
    );

    draw_star(
        grid,
        cols,
        rows,
        &light_ctx,
        stars,
        twinkle_stars_opt,
        time_elapsed,
        spotlights,
        &current_angles,
        &spot_cots,
        accent,
        is_secondary,
        intro_fade,
    );

    draw_dust(
        grid,
        cols,
        rows,
        &light_ctx,
        particles,
        spotlights,
        &current_angles,
        &spot_cots,
        accent,
        is_secondary,
        intro_fade,
    );

    if !is_secondary
        && let Some(logo) = crate::runner::place_centered_logo(cols, rows, logo_text, None)
    {
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
                if ch != ' ' && light_ctx.primary.contains(gx, gy) {
                    let (lr, lg, lb, intensity) = get_light_at(
                        gx as f32,
                        gy as f32,
                        &light_ctx,
                        spotlights,
                        &current_angles,
                        &spot_cots,
                        accent,
                        is_secondary,
                    );
                    let (fg_r, fg_g, fg_b) = if intensity > 0.05 {
                        let l_r = (90.0 * (1.0 - intensity) + lr * intensity).min(255.0) as u8;
                        let l_g = (20.0 * (1.0 - intensity) + lg * intensity).min(255.0) as u8;
                        let l_b = (120.0 * (1.0 - intensity) + lb * intensity).min(255.0) as u8;
                        (l_r, l_g, l_b)
                    } else {
                        (45, 20, 60)
                    };

                    let cell = &mut grid[gy * cols + gx];
                    cell.ch = ch;
                    cell.fg = (
                        (fg_r as f32 * intro_fade) as u8,
                        (fg_g as f32 * intro_fade) as u8,
                        (fg_b as f32 * intro_fade) as u8,
                    );
                    cell.bold = intensity > 0.05;
                }
            }
        }
    }
}
