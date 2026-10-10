//! Light and spotlight calculations for the beams screensaver.

use super::types::{Spotlight, smoothstep};
use crate::runner::MonitorCellBounds;
use crate::runner::TerminalCell;

/// Precomputed monitor/light geometry for a single draw pass.
pub struct LightContext {
    pub primary: MonitorCellBounds,
    y_origin: f32,
    inv_max_dist: f32,
    x_origins: Vec<f32>,
}

impl LightContext {
    pub fn new(cols: usize, rows: usize, spotlights: &[Spotlight]) -> Self {
        let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
        let y_origin = primary.end_row as f32;
        let inv_max_dist = 1.0 / (primary.height() as f32 * 1.6).max(1e-6);
        let x_origins = spotlights
            .iter()
            .map(|spot| primary.start_col as f32 + spot.origin_x_ratio * primary.width() as f32)
            .collect();
        Self {
            primary,
            y_origin,
            inv_max_dist,
            x_origins,
        }
    }
}

pub fn create_spotlights(beam_count: u32, accent: (u8, u8, u8), host_bias: f32) -> Vec<Spotlight> {
    let all_spots = super::types::default_spotlights();
    let mut spotlights = Vec::new();
    for i in 0..(beam_count as usize) {
        if i < all_spots.len() {
            let mut s = all_spots[i].clone();
            if i == 1 {
                s.color_r = accent.0 as f32;
                s.color_g = accent.1 as f32;
                s.color_b = accent.2 as f32;
            } else {
                let t = 0.18;
                s.color_r = s.color_r * (1.0 - t) + accent.0 as f32 * t;
                s.color_g = s.color_g * (1.0 - t) + accent.1 as f32 * t;
                s.color_b = s.color_b * (1.0 - t) + accent.2 as f32 * t;
            }
            s.motion_timer += host_bias * 2.5 + i as f32 * 0.35;
            spotlights.push(s);
        }
    }
    spotlights
}

/// Angular falloff: hard core + soft outer halo (smoothstep edges).
#[inline]
fn angular_weight(abs_da: f32, spread: f32) -> f32 {
    // Outer soft cone is ~1.75× the geometric spread.
    let outer = spread * 1.75;
    if abs_da >= outer {
        return 0.0;
    }
    // Soft penumbra between spread and outer.
    let soft = 1.0 - smoothstep(spread * 0.55, outer, abs_da);
    // Hotter core inside ~40% of spread.
    let core = 1.0 - smoothstep(0.0, spread * 0.45, abs_da);
    (soft * 0.72 + core * 0.55).min(1.2)
}

#[allow(clippy::too_many_arguments)]
pub fn get_light_at(
    cx: f32,
    cy: f32,
    ctx: &LightContext,
    spotlights: &[Spotlight],
    current_angles: &[f32],
    spot_cots: &[(f32, f32, f32, f32, f32)],
    accent: (u8, u8, u8),
    is_secondary: bool,
) -> (f32, f32, f32, f32) {
    // `is_secondary` is hoisted by callers: this runs per cell × beam —
    // querying the env/callback here was ~40k env scans per frame.
    if is_secondary {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mut r = 0.0f32;
    let mut g = 0.0f32;
    let mut b = 0.0f32;
    let mut total_intensity = 0.0f32;

    for (i, spot) in spotlights.iter().enumerate() {
        let x_origin = ctx.x_origins[i];
        let dx = (cx - x_origin) * 0.55;
        let dy = ctx.y_origin - cy;

        if dy > 0.0 {
            let (a_min, a_max, cot_min, cot_max, spread) = spot_cots[i];
            // Slightly wider culling for soft halo.
            let mut in_beam = true;
            if a_min > 1e-4 && dx >= dy * cot_min * 1.08 {
                in_beam = false;
            }
            if in_beam && a_max < std::f32::consts::PI - 1e-4 && dx <= dy * cot_max * 1.08 {
                in_beam = false;
            }

            if in_beam {
                let angle = dy.atan2(dx);
                let dist = (dx * dx + dy * dy).sqrt();
                let current_angle = current_angles[i];
                let mut da = angle - current_angle;

                if da > std::f32::consts::PI {
                    da -= std::f32::consts::TAU;
                    if da > std::f32::consts::PI {
                        da = (da + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                            - std::f32::consts::PI;
                    }
                } else if da < -std::f32::consts::PI {
                    da += std::f32::consts::TAU;
                    if da < -std::f32::consts::PI {
                        da = (da + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                            - std::f32::consts::PI;
                    }
                }

                let abs_da = da.abs();
                let ang = angular_weight(abs_da, spread);
                if ang > 0.0 {
                    // Soft distance falloff (quadratic-ish).
                    let dist_t = (dist * ctx.inv_max_dist).clamp(0.0, 1.0);
                    let dist_intensity = (1.0 - dist_t) * (1.0 - dist_t * 0.35);
                    let intensity = ang
                        * dist_intensity
                        * (0.92 + (spread / spot.spread.max(1e-6) - 1.0) * 0.4);

                    // Beam 1 tracks the live theme accent.
                    let (cr, cg, cb) = if i == 1 {
                        (accent.0 as f32, accent.1 as f32, accent.2 as f32)
                    } else {
                        (spot.color_r, spot.color_g, spot.color_b)
                    };
                    r += intensity * cr;
                    g += intensity * cg;
                    b += intensity * cb;
                    total_intensity += intensity;
                }
            }
        }
    }
    (
        r.min(255.0),
        g.min(255.0),
        b.min(255.0),
        total_intensity.min(1.0),
    )
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_spotlight(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    ctx: &LightContext,
    spotlights: &[Spotlight],
    current_angles: &[f32],
    spot_cots: &[(f32, f32, f32, f32, f32)],
    accent: (u8, u8, u8),
    is_secondary: bool,
    intro_fade: f32,
) {
    let fade = intro_fade.clamp(0.0, 1.0);
    for y in 0..rows {
        let y_f = y as f32;
        let y_cols = y * cols;
        for x in 0..cols {
            let (bg_r, bg_g, bg_b) = if is_secondary {
                (0, 0, 0)
            } else {
                let (r, g, b, intensity) = get_light_at(
                    x as f32,
                    y_f,
                    ctx,
                    spotlights,
                    current_angles,
                    spot_cots,
                    accent,
                    is_secondary,
                );
                let on_primary = ctx.primary.contains(x, y);
                // Soft outer glow even at low intensity
                let show_glow = on_primary || intensity > 0.02;
                let glow = 0.12 + intensity * 0.10;
                let bg_r = if show_glow {
                    (r * glow * fade) as u8
                } else {
                    0
                };
                let bg_g = if show_glow {
                    (g * glow * fade) as u8
                } else {
                    0
                };
                let bg_b = if show_glow {
                    (b * glow * fade) as u8
                } else {
                    0
                };
                (bg_r, bg_g, bg_b)
            };

            grid[y_cols + x] = TerminalCell {
                ch: ' ',
                fg: (0, 0, 0),
                bg: (bg_r, bg_g, bg_b),
                bold: false,
            };
        }
    }
}
