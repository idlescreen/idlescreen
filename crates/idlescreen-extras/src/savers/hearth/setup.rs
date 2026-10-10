//! Initialization and setup logic for Hearth screensaver.

use super::types::{Log, Tongue};
use crate::runner::LcgRng;

pub struct HearthLayout {
    pub fire_cx: f32,
    pub fire_y: f32,
    pub fire_w: f32,
    pub mantel_y: usize,
    pub logs: Vec<Log>,
    pub tongues: Vec<Tongue>,
}

pub fn create_layout(rng: &mut LcgRng, cols: usize, rows: usize) -> HearthLayout {
    let fire_cx = cols as f32 * 0.5;
    let fire_y = (rows as f32 * 0.82).floor().max(8.0);
    // `[saver] hearth.fire_size` (or global `fire_size`) scales flame width;
    // default 1.0. Clamped so a hostile/degenerate value can't break layout.
    let fire_size = crate::runner::param_f32("hearth.fire_size")
        .or_else(|| crate::runner::param_f32("fire_size"))
        .unwrap_or(1.0)
        .clamp(0.5, 2.0);
    let fire_w = (cols as f32 * 0.36 * fire_size).clamp(16.0, 52.0);
    // rows < 9 makes the upper clamp < 3 (panic) — floor it, then keep the
    // result inside the grid for degenerate heights.
    let mantel_y = ((rows as f32 * 0.28).round() as usize)
        .clamp(3, rows.saturating_sub(6).max(3))
        .min(rows.saturating_sub(1));

    let mut logs = Vec::new();
    let log_y = (fire_y as usize).saturating_sub(1);
    let log_w = (fire_w * 0.42).round() as usize;
    let log_w = log_w.max(6);
    let cx = fire_cx as usize;
    if cx > log_w / 2 && cols > cx + log_w / 2 {
        logs.push(Log {
            x: cx - log_w / 2,
            y: log_y,
            w: log_w,
            phase: rng.next_f32() * std::f32::consts::TAU,
        });
    }
    let secondary_w = (log_w as f32 * 0.75) as usize;
    if cx > secondary_w && cols > cx + secondary_w / 2 {
        logs.push(Log {
            x: cx - secondary_w + 1,
            y: log_y.saturating_sub(1),
            w: secondary_w,
            phase: rng.next_f32() * std::f32::consts::TAU,
        });
    }

    let mut tongues = Vec::new();
    let offsets = [-0.65f32, -0.32, 0.0, 0.32, 0.65];
    for &off in &offsets {
        tongues.push(Tongue {
            offset: off + (rng.next_f32() - 0.5) * 0.1,
            phase: rng.next_f32() * std::f32::consts::TAU,
            speed: 1.8 + rng.next_f32() * 1.4,
            height_scale: 0.75 + rng.next_f32() * 0.5,
            width_scale: 0.85 + rng.next_f32() * 0.35,
        });
    }

    HearthLayout {
        fire_cx,
        fire_y,
        fire_w,
        mantel_y,
        logs,
        tongues,
    }
}
