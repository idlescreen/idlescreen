//! Curtain seeding and the per-column edge/ray field evaluation.

use super::types::{Curtain, Star};
use crate::runner::LcgRng;

/// Three base curtains plus a fourth slot reserved for the surge wave
/// (its `strength` is driven by `surge_env` each frame).
pub fn seed_curtains(rng: &mut LcgRng, host_bias: f32) -> Vec<Curtain> {
    let wobble = |r: &mut LcgRng, lo: f32, hi: f32| r.next_range(lo, hi);
    vec![
        Curtain {
            base_y: 0.30 + host_bias * 0.04,
            amp: wobble(rng, 0.055, 0.085),
            freq: wobble(rng, 0.075, 0.105),
            speed: wobble(rng, 0.35, 0.55),
            phase: rng.next_f32() * std::f32::consts::TAU,
            amp2: wobble(rng, 0.018, 0.030),
            freq2: wobble(rng, 0.20, 0.30),
            sigma: wobble(rng, 4.5, 6.5),
            hue_mix: wobble(rng, 0.0, 0.15),
            strength: 1.0,
        },
        Curtain {
            base_y: 0.40 + host_bias * 0.04,
            amp: wobble(rng, 0.045, 0.070),
            freq: wobble(rng, 0.10, 0.14),
            speed: wobble(rng, 0.45, 0.70),
            phase: rng.next_f32() * std::f32::consts::TAU,
            amp2: wobble(rng, 0.015, 0.028),
            freq2: wobble(rng, 0.24, 0.36),
            sigma: wobble(rng, 3.5, 5.5),
            hue_mix: wobble(rng, 0.35, 0.60),
            strength: 0.85,
        },
        Curtain {
            base_y: 0.50 + host_bias * 0.04,
            amp: wobble(rng, 0.035, 0.055),
            freq: wobble(rng, 0.13, 0.19),
            speed: wobble(rng, 0.55, 0.85),
            phase: rng.next_f32() * std::f32::consts::TAU,
            amp2: wobble(rng, 0.012, 0.022),
            freq2: wobble(rng, 0.28, 0.42),
            sigma: wobble(rng, 2.8, 4.2),
            hue_mix: wobble(rng, 0.75, 1.0),
            strength: 0.7,
        },
        // Surge curtain — fast, high, bright; strength modulated per frame.
        Curtain {
            base_y: 0.24,
            amp: wobble(rng, 0.09, 0.13),
            freq: wobble(rng, 0.06, 0.09),
            speed: wobble(rng, 1.1, 1.5),
            phase: rng.next_f32() * std::f32::consts::TAU,
            amp2: wobble(rng, 0.02, 0.04),
            freq2: wobble(rng, 0.18, 0.26),
            sigma: wobble(rng, 5.5, 8.0),
            hue_mix: wobble(rng, 0.05, 0.25),
            strength: 0.0,
        },
    ]
}

/// Sparse static starfield across the sky band. Density scales with
/// grid area and adaptive quality.
pub fn seed_stars(rng: &mut LcgRng, cols: usize, rows: usize, quality: f32) -> Vec<Star> {
    let target = (((cols * rows) / 110).clamp(12, 90) as f32 * quality) as usize;
    (0..target)
        .map(|i| Star {
            x: rng.next_f32(),
            y: rng.next_f32() * 0.85,
            phase: rng.next_f32() * std::f32::consts::TAU,
            rate: rng.next_range(0.6, 2.4),
            ch: if i.is_multiple_of(9) {
                '✦'
            } else if i.is_multiple_of(3) {
                '+'
            } else {
                '·'
            },
        })
        .collect()
}

/// Per-column curtain edge heights and ray modulation. Evaluating edges
/// once per column turns the per-cell loop into O(1) multiplies instead
/// of per-cell sin() calls.
pub fn eval_columns(
    curtains: &[Curtain],
    t: f32,
    cols: usize,
    rows: usize,
    quality: f32,
    edges: &mut Vec<f32>,
    rays: &mut Vec<f32>,
) {
    let n = curtains.len();
    edges.clear();
    edges.resize(cols * n, 0.0);
    rays.clear();
    rays.resize(cols * n, 0.0);
    if cols == 0 || rows == 0 {
        return;
    }
    let rows_f = rows as f32;
    let harmonics = quality >= 0.5;
    for (i, c) in curtains.iter().enumerate() {
        if c.strength <= 0.001 {
            continue;
        }
        let base = c.base_y * rows_f;
        let lo = -0.15 * rows_f;
        let hi = 1.10 * rows_f;
        for x in 0..cols {
            let xf = x as f32;
            let mut e = base + rows_f * c.amp * (c.freq * xf + c.speed * t + c.phase).sin();
            if harmonics {
                e += rows_f * c.amp2 * (c.freq2 * xf - c.speed * 0.63 * t).sin();
            }
            edges[i * cols + x] = e.clamp(lo, hi);
            // Vertical striation rays: column-constant brightness ripple.
            rays[i * cols + x] = 0.72 + 0.28 * (xf * 0.55 + t * 1.3 + i as f32 * 2.39).sin();
        }
    }
}
