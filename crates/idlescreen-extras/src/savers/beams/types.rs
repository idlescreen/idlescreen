//! Auxiliary types and defaults for the beams screensaver.

use std::f32::consts::FRAC_PI_2;

#[derive(Clone)]
pub struct Spotlight {
    pub origin_x_ratio: f32,
    pub angle_center: f32,
    pub angle_amplitude: f32,
    pub speed: f32,
    pub phase_offset: f32,
    pub phase: f32,
    pub spread: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    /// Per-beam speed scale (desync so cones don't look mechanical).
    pub speed_bias: f32,
    /// 1.0 = full sweep, ~0.08 = brief rest. Per-beam so not all freeze at once.
    pub motion_blend: f32,
    /// Seconds until next active↔calm flip for this beam.
    pub motion_timer: f32,
    /// True while this beam is in a calm rest.
    pub is_calm: bool,
}

pub struct Star {
    pub x: f32,
    pub y: f32,
    pub phase: f32,
    pub ch: char,
    pub excitation: f32,
    /// 0 = far (dim, slow twinkle), 1 = near
    pub layer: u8,
}

pub struct DustParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// Rest vertical speed (negative = up the screen).
    pub base_vy: f32,
    /// 0 = far plane, 1 = near plane
    pub layer: u8,
    /// Brief spark when crossing a bright ridge
    pub spark: f32,
}

/// Softstep 0..1 (Hermite).
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn default_spotlights() -> Vec<Spotlight> {
    // Speeds / phases deliberately uneven so motion never looks like a clockwork loop.
    // motion_timer offsets are staggered so rests never land on every beam at once.
    vec![
        Spotlight {
            origin_x_ratio: 0.15,
            angle_center: FRAC_PI_2,
            angle_amplitude: 0.55,
            speed: 0.72,
            phase_offset: 0.0,
            phase: 0.35,
            spread: 0.16,
            color_r: 160.0,
            color_g: 20.0,
            color_b: 255.0,
            speed_bias: 0.92,
            motion_blend: 1.0,
            motion_timer: 4.0,
            is_calm: false,
        },
        Spotlight {
            origin_x_ratio: 0.50,
            angle_center: FRAC_PI_2,
            angle_amplitude: 0.70,
            speed: 0.48,
            phase_offset: 2.15,
            phase: 1.1,
            spread: 0.14,
            color_r: 0.0,
            color_g: 130.0,
            color_b: 255.0,
            speed_bias: 1.18,
            motion_blend: 1.0,
            motion_timer: 7.5,
            is_calm: false,
        },
        Spotlight {
            origin_x_ratio: 0.85,
            angle_center: FRAC_PI_2,
            angle_amplitude: 0.60,
            speed: 0.95,
            phase_offset: 4.7,
            phase: 2.4,
            spread: 0.15,
            color_r: 255.0,
            color_g: 0.0,
            color_b: 130.0,
            speed_bias: 0.85,
            motion_blend: 1.0,
            motion_timer: 11.0,
            is_calm: false,
        },
        Spotlight {
            origin_x_ratio: 0.30,
            angle_center: FRAC_PI_2,
            angle_amplitude: 0.68,
            speed: 0.58,
            phase_offset: 1.05,
            phase: 0.7,
            spread: 0.145,
            color_r: 0.0,
            color_g: 220.0,
            color_b: 180.0,
            speed_bias: 1.05,
            motion_blend: 1.0,
            motion_timer: 6.0,
            is_calm: false,
        },
        Spotlight {
            origin_x_ratio: 0.70,
            angle_center: FRAC_PI_2,
            angle_amplitude: 0.65,
            speed: 0.80,
            phase_offset: 3.4,
            phase: 1.8,
            spread: 0.15,
            color_r: 255.0,
            color_g: 170.0,
            color_b: 0.0,
            speed_bias: 0.98,
            motion_blend: 1.0,
            motion_timer: 9.5,
            is_calm: false,
        },
        Spotlight {
            origin_x_ratio: 0.40,
            angle_center: FRAC_PI_2,
            angle_amplitude: 0.58,
            speed: 0.52,
            phase_offset: 5.2,
            phase: 3.0,
            spread: 0.13,
            color_r: 220.0,
            color_g: 0.0,
            color_b: 220.0,
            speed_bias: 1.12,
            motion_blend: 1.0,
            motion_timer: 13.0,
            is_calm: false,
        },
    ]
}
