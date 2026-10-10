//! Beam motion and particle spawning helpers for Beams screensaver.

use crate::beams::MAX_CALM_BEAMS;
use crate::beams::types::{DustParticle, Spotlight, Star};
use crate::runner::LcgRng;

pub fn spawn_dust(rng: &mut LcgRng, near: bool) -> DustParticle {
    let layer = u8::from(near);
    let base_vy = if near {
        -rng.next_range(0.06, 0.14)
    } else {
        -rng.next_range(0.02, 0.06)
    };
    DustParticle {
        x: rng.next_f32(),
        y: rng.next_f32(),
        vx: rng.next_range(-0.03, 0.03) * if near { 1.0 } else { 0.45 },
        vy: base_vy,
        base_vy,
        layer,
        spark: 0.0,
    }
}

pub fn spawn_star(rng: &mut LcgRng, stars_len: usize, near: bool) -> Star {
    Star {
        x: rng.next_f32(),
        y: rng.next_f32(),
        phase: rng.next_f32() * std::f32::consts::TAU,
        ch: if near {
            if stars_len.is_multiple_of(8) {
                '✦'
            } else if stars_len.is_multiple_of(3) {
                '+'
            } else {
                '.'
            }
        } else {
            '.'
        },
        excitation: 0.0,
        layer: u8::from(near),
    }
}

/// Per-beam breathing: at most [`MAX_CALM_BEAMS`] rest together so the field
/// never freezes.
pub fn update_beam_motion(spotlights: &mut [Spotlight], rng: &mut LcgRng, delta: f32) {
    let mut calm_count = spotlights.iter().filter(|s| s.is_calm).count();

    for spot in spotlights {
        spot.motion_timer -= delta;
        if spot.motion_timer <= 0.0 {
            if spot.is_calm {
                spot.is_calm = false;
                calm_count = calm_count.saturating_sub(1);
                spot.motion_timer = 5.0 + rng.next_f32() * 8.0;
            } else if calm_count < MAX_CALM_BEAMS {
                spot.is_calm = true;
                calm_count += 1;
                spot.motion_timer = 1.2 + rng.next_f32() * 1.6;
            } else {
                spot.motion_timer = 1.5 + rng.next_f32() * 3.5;
            }
        }

        let target = if spot.is_calm { 0.08 } else { 1.0 };
        let ease = 1.0 - (-delta * 1.8).exp();
        spot.motion_blend += (target - spot.motion_blend) * ease;
    }
}
