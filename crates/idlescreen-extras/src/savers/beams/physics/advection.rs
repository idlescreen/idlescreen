//! Dust particle advection and light sampling logic for Beams.

use crate::beams::light;
use crate::beams::types::{DustParticle, Spotlight, Star};
use crate::runner::LcgRng;

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn update_dust_and_stars(
    particles: &mut [DustParticle],
    stars: &mut [Star],
    spotlights: &[Spotlight],
    rng: &mut LcgRng,
    cols: usize,
    rows: usize,
    delta: f32,
    accent: (u8, u8, u8),
    is_secondary: bool,
) {
    let mut angles = Vec::with_capacity(spotlights.len());
    let mut cots = Vec::with_capacity(spotlights.len());
    for spot in spotlights {
        let blend = spot.motion_blend;
        let angle = spot.angle_center
            + spot.angle_amplitude * (spot.phase + spot.phase_offset).sin() * (0.35 + 0.65 * blend);
        angles.push(angle);
        let a_min = angle - spot.spread * 1.75;
        let a_max = angle + spot.spread * 1.75;
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
        cots.push((a_min, a_max, cot_min, cot_max, 1.0 / spot.spread.max(1e-6)));
    }
    let light_ctx = light::LightContext::new(cols, rows, spotlights);

    let cols_f = cols as f32;
    let rows_f = rows as f32;
    for p in particles {
        let px = p.x * cols_f;
        let py = p.y * rows_f;
        let (_, _, _, intensity) = light::get_light_at(
            px,
            py,
            &light_ctx,
            spotlights,
            &angles,
            &cots,
            accent,
            is_secondary,
        );

        let updraft = 1.0 + intensity * 2.2;
        let side = (intensity - 0.35).max(0.0) * rng.next_range(-0.02, 0.02);
        p.vy = p.base_vy * updraft * (0.55 + 0.45 * if p.layer == 1 { 1.0 } else { 0.6 });
        p.vx = (p.vx * 0.98) + side;

        if intensity > 0.55 && p.spark <= 0.0 && rng.next_f32() < 0.04 * intensity {
            p.spark = 0.35 + rng.next_f32() * 0.25;
        }
        if p.spark > 0.0 {
            p.spark = (p.spark - delta * 2.8).max(0.0);
        }

        p.x += p.vx * delta;
        p.y += p.vy * delta;

        if p.y < 0.0 {
            p.y = 1.0;
            p.x = rng.next_f32();
            p.spark = 0.0;
        }
        if p.x < 0.0 {
            p.x = 1.0;
        } else if p.x > 1.0 {
            p.x = 0.0;
        }

        for star in stars.iter_mut() {
            let dx = (p.x - star.x) * cols_f;
            let dy = (p.y - star.y) * rows_f * 2.0;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < 6.25 {
                let dist = dist_sq.sqrt();
                let force = (1.0 - dist / 2.5) * 1.5;
                let layer_boost = if p.layer == 1 && star.layer == 1 {
                    1.0
                } else {
                    0.45
                };
                star.excitation = star.excitation.max(force * layer_boost);
            }
        }
    }
}
