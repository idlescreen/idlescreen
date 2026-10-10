//! Core calculations and helper functions for the Glyphs screensaver.

use crate::glyphs::RainDrop;
use crate::runner::LcgRng;

/// Calculates the foreground color of a glyph based on its position in the tail.
/// Strong bright head → deep dim trail; far layers stay softer overall.
pub fn calculate_glyph_color(
    accent: (u8, u8, u8),
    k: usize,
    length: usize,
    layer: u8,
) -> (u8, u8, u8) {
    let t = if length <= 1 {
        1.0
    } else {
        1.0 - (k as f32 / (length as f32 - 1.0).max(1.0))
    };
    // Steeper head/trail gradient: head ~full, mid ~0.25, tip ~0.06
    let intensity = if k == 0 {
        1.0
    } else {
        (0.06 + 0.55 * t.powi(2)).clamp(0.05, 0.72)
    };
    let depth = if layer == 0 { 0.42 } else { 1.0 };
    let scale = intensity * depth;
    let r = ((accent.0 as f32) * scale) as u8;
    let g = ((accent.1 as f32) * scale) as u8;
    let b = ((accent.2 as f32) * scale) as u8;
    (r, g, b)
}

/// Updates a single raindrop's position and speed (with gentle per-drop breathing).
pub fn update_drop(
    d: &mut RainDrop,
    delta: f32,
    rows: usize,
    char_pool_len: usize,
    time_elapsed: f32,
    rng: &mut LcgRng,
) {
    let breathe = 0.88 + 0.14 * (time_elapsed * 0.55 + d.speed_phase).sin();
    d.y += d.speed * delta * breathe;
    d.char_rot = (d.char_rot + 1) % char_pool_len.max(1);
    if d.y as i32 > rows as i32 + d.length as i32 {
        *d = spawn_drop(d.x as usize, rows, char_pool_len, rng);
        // Keep column; re-roll only depth/motion identity.
        d.y = -(d.length as f32) - rng.next_f32() * 4.0;
    }
}

/// Helper to initialize/spawn a raindrop with near/far depth.
pub fn spawn_drop(x: usize, rows: usize, char_pool_len: usize, rng: &mut LcgRng) -> RainDrop {
    // ~40% far plane for soft depth behind the bright columns.
    let layer: u8 = u8::from(rng.next_f32() >= 0.40);
    let (speed_lo, speed_hi, len_extra) = if layer == 0 {
        (5.0, 14.0, 10usize)
    } else {
        (12.0, 32.0, 16usize)
    };
    RainDrop {
        x: x as f32,
        y: -rng.next_f32() * (rows as f32),
        speed: rng.next_range(speed_lo, speed_hi),
        length: 4 + rng.next_usize(len_extra),
        char_rot: rng.next_usize(char_pool_len.max(1)),
        layer,
        speed_phase: rng.next_f32() * std::f32::consts::TAU,
    }
}
