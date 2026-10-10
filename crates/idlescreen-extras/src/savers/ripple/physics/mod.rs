//! Physics simulation and displacement helpers for Ripple.

pub mod update;

use super::types::Ring;

/// Radial displacement from active ring fronts at a point (logo warp).
/// Returns (dx, dy, hit_strength 0..~2).
pub fn wave_displace(x: f32, y: f32, rings: &[Ring]) -> (f32, f32, f32) {
    let mut dx = 0.0f32;
    let mut dy = 0.0f32;
    let mut peak = 0.0f32;
    let aspect = 0.5f32;
    for ring in rings {
        let ox = x - ring.x;
        let oy = (y - ring.y) / aspect;
        let d = (ox * ox + oy * oy).sqrt();
        let front = (d - ring.r).abs();
        let band = 2.8f32;
        if front >= band {
            continue;
        }
        let w = (1.0 - front / band) * ring.life * ring.strength;
        if w < 0.04 {
            continue;
        }
        let len = d.max(0.15);
        let amp = w * 2.2;
        dx += (ox / len) * amp;
        dy += (oy / len) * amp * aspect;
        peak = peak.max(w);
    }
    (dx, dy, peak)
}

/// Stroke an ellipse into a brightness accum buffer.
// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn stroke_ellipse(
    accum: &mut [f32],
    accent_accum: &mut [f32],
    cols: usize,
    rows: usize,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    strength: f32,
    accent_w: f32,
    wind_skew: f32,
) {
    if rx < 0.4 || ry < 0.25 || strength < 0.02 {
        return;
    }
    let steps = ((rx.max(ry) * 10.0).clamp(16.0, 160.0) as usize).max(12);
    let mut prev: Option<(isize, isize)> = None;
    for i in 0..=steps {
        let theta = (i as f32 / steps as f32) * std::f32::consts::TAU;
        let px = cx + theta.cos() * rx + wind_skew * ry * 0.15;
        let py = cy + theta.sin() * ry;
        let x = px.round() as isize;
        let y = py.round() as isize;
        if let Some((px0, py0)) = prev {
            plot_line(
                accum,
                accent_accum,
                cols,
                rows,
                px0,
                py0,
                x,
                y,
                strength,
                accent_w,
            );
        } else {
            plot_point(accum, accent_accum, cols, rows, x, y, strength, accent_w);
        }
        prev = Some((x, y));
    }
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn plot_point(
    accum: &mut [f32],
    accent_accum: &mut [f32],
    cols: usize,
    rows: usize,
    x: isize,
    y: isize,
    strength: f32,
    accent_w: f32,
) {
    if x < 0 || y < 0 || x >= cols as isize || y >= rows as isize {
        return;
    }
    let i = y as usize * cols + x as usize;
    accum[i] = (accum[i] + strength).min(2.5);
    if accent_w > 0.0 {
        accent_accum[i] = (accent_accum[i] + strength * accent_w).min(2.0);
    }
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn plot_line(
    accum: &mut [f32],
    accent_accum: &mut [f32],
    cols: usize,
    rows: usize,
    x0: isize,
    y0: isize,
    x1: isize,
    y1: isize,
    strength: f32,
    accent_w: f32,
) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        plot_point(accum, accent_accum, cols, rows, x, y, strength, accent_w);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
#[path = "physics_tests.rs"]
mod tests;
