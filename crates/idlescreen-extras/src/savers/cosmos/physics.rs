pub mod bounds;
pub mod drift;
pub mod history;
pub mod logo;
pub mod merges;
pub mod particle_cap;
pub mod states;
pub mod stellar;
pub mod update;

#[allow(unused_imports)]
pub use stellar::{accretion, collapse, expansion, gravitation, ignition, singularity};

pub use states::enter_state;
pub use update::update_frame_time;
pub use update::update_life;

use crate::runner::span_reach_scale;

/// Camera scale for the particle field (logo uses its own `logo_scale`).
pub fn default_zoom(cols: usize, rows: usize) -> f32 {
    let reach = span_reach_scale(cols, rows);
    (0.58 / reach.sqrt()).clamp(0.42, 0.72)
}

/// Terminal cells are ~2× wider than tall; scale vertical screen deltas for round metrics.
pub const TERMINAL_ASPECT_Y: f32 = 2.0;

/// Screen cell → logo universe (1:1 screen offset from center, no zoom).
pub fn screen_to_logo_universe(
    sx: f32,
    sy: f32,
    universe_cx: f32,
    universe_cy: f32,
    screen_cx: f32,
    screen_cy: f32,
) -> (f32, f32) {
    (
        universe_cx + (sx - screen_cx),
        universe_cy + (sy - screen_cy),
    )
}

/// Logo universe → particle universe (zoom-scaled simulation space).
pub fn logo_to_particle_universe(
    lp_x: f32,
    lp_y: f32,
    universe_cx: f32,
    universe_cy: f32,
    zoom: f32,
) -> (f32, f32) {
    let z = zoom.max(0.2);
    (
        universe_cx + (lp_x - universe_cx) / z,
        universe_cy + (lp_y - universe_cy) / z,
    )
}

/// Logo pixels live in screen-cell offsets from universe center (no zoom).
pub fn logo_to_screen_fast(
    lp_x: f32,
    lp_y: f32,
    universe_cx: f32,
    universe_cy: f32,
    screen_cx: f32,
    screen_cy: f32,
) -> (i32, i32) {
    let sx = screen_cx + (lp_x - universe_cx);
    let sy = screen_cy + (lp_y - universe_cy);
    (sx.round() as i32, sy.round() as i32)
}

pub fn to_screen_fast(
    ux: f32,
    uy: f32,
    universe_cx: f32,
    universe_cy: f32,
    screen_cx: f32,
    screen_cy: f32,
    zoom: f32,
) -> (i32, i32) {
    let sx = screen_cx + (ux - universe_cx) * zoom;
    let sy = screen_cy + (uy - universe_cy) * zoom;
    (sx.round() as i32, sy.round() as i32)
}
