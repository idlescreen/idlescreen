//! Struct definitions and types for Ripple screensaver.

use crate::runner::LcgRng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Lull,
    Drizzle,
    Shower,
}

pub struct Ring {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub max_r: f32,
    pub life: f32,
    pub strength: f32,
    pub age: f32,
}

pub struct Drop {
    pub x: f32,
    pub y: f32,
    pub vy: f32,
    pub target_y: f32,
}

pub struct Splash {
    pub x: f32,
    pub y: f32,
    pub life: f32,
}

pub struct DelayedRing {
    pub x: f32,
    pub y: f32,
    pub strength: f32,
    pub delay: f32,
}

pub fn max_rings(weather: Weather, on_battery: bool, quality_scale: f32) -> usize {
    let base = match weather {
        Weather::Lull => 12.0,
        Weather::Drizzle => 28.0,
        Weather::Shower => 48.0,
    };
    let bat = if on_battery { 0.7 } else { 1.0 };
    (base * quality_scale * bat) as usize
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn spawn_ring(
    rings: &mut Vec<Ring>,
    weather: Weather,
    on_battery: bool,
    quality_scale: f32,
    x: f32,
    y: f32,
    strength: f32,
    cols: usize,
    rows: usize,
) {
    if rings.len() >= max_rings(weather, on_battery, quality_scale) {
        return;
    }
    let weather_scale = match weather {
        Weather::Lull => 0.75,
        Weather::Drizzle => 1.0,
        Weather::Shower => 1.25,
    };
    let max_r = (8.0 + strength * 18.0) * (0.7 + 0.3 * quality_scale) * weather_scale;
    rings.push(Ring {
        x: x.clamp(0.0, cols.saturating_sub(1) as f32),
        y: y.clamp(0.0, rows.saturating_sub(1) as f32),
        r: 0.25,
        max_r,
        life: 1.0,
        strength: strength.clamp(0.3, 1.0),
        age: 0.0,
    });
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn impact(
    rings: &mut Vec<Ring>,
    splashes: &mut Vec<Splash>,
    delayed: &mut Vec<DelayedRing>,
    rng: &mut LcgRng,
    weather: Weather,
    on_battery: bool,
    quality_scale: f32,
    x: f32,
    y: f32,
    strength: f32,
    cols: usize,
    rows: usize,
) {
    spawn_ring(
        rings,
        weather,
        on_battery,
        quality_scale,
        x,
        y,
        strength,
        cols,
        rows,
    );
    splashes.push(Splash {
        x,
        y,
        life: 0.18 + strength * 0.12,
    });
    if strength > 0.45 && rng.next_f32() < 0.7 {
        delayed.push(DelayedRing {
            x: x + (rng.next_f32() - 0.5) * 1.5,
            y: y + (rng.next_f32() - 0.5) * 0.8,
            strength: strength * (0.35 + rng.next_f32() * 0.25),
            delay: 0.12 + rng.next_f32() * 0.22,
        });
    }
    if strength > 0.75 && rng.next_f32() < 0.4 {
        delayed.push(DelayedRing {
            x,
            y,
            strength: strength * 0.25,
            delay: 0.35 + rng.next_f32() * 0.2,
        });
    }
}
