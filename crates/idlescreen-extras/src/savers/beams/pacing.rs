//! Frame time and quality scale pacing logic for Beams.

use std::time::Duration;

pub struct FramePacing {
    pub frame_time_ema: f32,
    pub target_frame_time: f32,
    pub quality_scale: f32,
}

pub fn update_frame_time(
    pacing: &mut FramePacing,
    time_elapsed: f32,
    on_battery: bool,
    dt: Duration,
) {
    let dt_secs = dt.as_secs_f32();

    if time_elapsed < 2.0 && dt_secs > 0.001 && dt_secs < pacing.target_frame_time - 0.001 {
        pacing.target_frame_time = dt_secs;
    }

    pacing.frame_time_ema = pacing.frame_time_ema * 0.9 + dt_secs.min(0.2) * 0.1;

    if time_elapsed > 1.5 {
        let speed_mult = if on_battery { 0.65 } else { 1.0 };
        let delta = dt_secs * speed_mult;
        if pacing.frame_time_ema > pacing.target_frame_time * 1.15 {
            pacing.quality_scale = (pacing.quality_scale - 0.15 * delta).max(0.20);
        } else if pacing.frame_time_ema < pacing.target_frame_time * 1.05 {
            pacing.quality_scale = (pacing.quality_scale + 0.04 * delta).min(1.0);
        }
    }
}
