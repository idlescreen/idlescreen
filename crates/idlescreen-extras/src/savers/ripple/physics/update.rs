//! Simulation update logic for Ripple screensaver.

use crate::ripple::types::{self, DelayedRing, Drop, Ring, Splash, Weather};
use crate::runner::LcgRng;

pub struct RippleState<'a> {
    pub rings: &'a mut Vec<Ring>,
    pub drops: &'a mut Vec<Drop>,
    pub splashes: &'a mut Vec<Splash>,
    pub delayed: &'a mut Vec<DelayedRing>,
    pub wind: &'a mut f32,
    pub weather: &'a mut Weather,
    pub weather_intensity: &'a mut f32,
    pub rain_timer: &'a mut f32,
}

pub fn update_simulation(
    state: RippleState,
    rng: &mut LcgRng,
    cols: usize,
    rows: usize,
    quality_scale: f32,
    on_battery: bool,
    delta: f32,
) {
    let weather = *state.weather;
    let target = match weather {
        Weather::Lull => 0.15,
        Weather::Drizzle => 0.45,
        Weather::Shower => 0.9,
    };
    *state.weather_intensity += (target - *state.weather_intensity) * (delta * 0.6);

    *state.wind += (rng.next_f32() - 0.5) * delta * 0.4;
    *state.wind = state.wind.clamp(-1.2, 1.2);

    for r in state.rings.iter_mut() {
        r.age += delta;
        r.r += delta * (4.5 + 3.0 * r.strength) * quality_scale;
        r.life = 1.0 - (r.r / r.max_r).clamp(0.0, 1.0);
        r.x += *state.wind * delta * 0.8;
    }
    state
        .rings
        .retain(|r| r.life > 0.02 && r.r < r.max_r * 1.05);

    let mut due = Vec::new();
    for d in state.delayed.iter_mut() {
        d.delay -= delta;
        if d.delay <= 0.0 {
            due.push((d.x, d.y, d.strength));
        }
    }
    state.delayed.retain(|d| d.delay > 0.0);
    for (x, y, s) in due {
        types::spawn_ring(
            state.rings,
            weather,
            on_battery,
            quality_scale,
            x,
            y,
            s,
            cols,
            rows,
        );
    }

    let mut impacts = Vec::new();
    for d in state.drops.iter_mut() {
        d.y += d.vy * delta;
        d.x += *state.wind * delta * 2.5;
        if d.y >= d.target_y {
            impacts.push((d.x, d.target_y, 0.45 + rng.next_f32() * 0.4));
        }
    }
    state.drops.retain(|d| d.y < d.target_y);
    for (x, y, s) in impacts {
        types::impact(
            state.rings,
            state.splashes,
            state.delayed,
            rng,
            weather,
            on_battery,
            quality_scale,
            x,
            y,
            s,
            cols,
            rows,
        );
    }

    for s in state.splashes.iter_mut() {
        s.life -= delta;
    }
    state.splashes.retain(|s| s.life > 0.0);

    *state.rain_timer -= delta;
    let base_interval = match weather {
        Weather::Lull => 0.55,
        Weather::Drizzle => 0.12,
        Weather::Shower => 0.045,
    };
    let bat = if on_battery { 1.4 } else { 1.0 };
    let interval = base_interval * bat / quality_scale.max(0.3);

    while *state.rain_timer <= 0.0 {
        *state.rain_timer += interval * (0.5 + rng.next_f32());
        let hit_chance = 0.35 + *state.weather_intensity * 0.55;
        let x = rng.next_f32() * cols as f32;
        if rng.next_f32() < hit_chance {
            if rng.next_f32() < 0.55 + *state.weather_intensity * 0.2 {
                let y = rng.next_f32() * rows as f32;
                let strength = 0.3 + rng.next_f32() * (0.4 + *state.weather_intensity * 0.4);
                types::impact(
                    state.rings,
                    state.splashes,
                    state.delayed,
                    rng,
                    weather,
                    on_battery,
                    quality_scale,
                    x,
                    y,
                    strength,
                    cols,
                    rows,
                );
            } else if state.drops.len() < 20 + (*state.weather_intensity * 20.0) as usize {
                let target_y = rows as f32 * (0.2 + rng.next_f32() * 0.75);
                state.drops.push(Drop {
                    x,
                    y: -1.0 - rng.next_f32() * 4.0,
                    vy: 18.0 + rng.next_f32() * 22.0,
                    target_y,
                });
            }
        }
    }
}
