//! Ember and smoke particle updating logic for Hearth.

use super::types::{Ember, Smoke};
use crate::runner::LcgRng;

pub fn spawn_ember(
    embers: &mut Vec<Ember>,
    rng: &mut LcgRng,
    fire_cx: f32,
    fire_y: f32,
    fire_w: f32,
    cols: usize,
) {
    let x_offset = (rng.next_f32() - 0.5) * fire_w * 0.75;
    let x = fire_cx + x_offset;
    let vy = -0.6 - rng.next_f32() * 1.8;
    let vx = (rng.next_f32() - 0.5) * 0.9;
    let life = 1.2 + rng.next_f32() * 2.8;
    let size = if rng.next_f32() < 0.28 { 2 } else { 1 };
    embers.push(Ember {
        x: x.clamp(0.0, cols.saturating_sub(1) as f32),
        y: fire_y - rng.next_f32() * 1.2,
        vx,
        vy,
        life,
        max_life: life,
        size,
        heat: 0.85 + rng.next_f32() * 0.15,
    });
}

pub fn spawn_burst(
    embers: &mut Vec<Ember>,
    rng: &mut LcgRng,
    fire_cx: f32,
    fire_y: f32,
    fire_w: f32,
    cols: usize,
) {
    if rng.next_f32() < 0.18 {
        for _ in 0..3 {
            spawn_ember(embers, rng, fire_cx, fire_y, fire_w, cols);
            if let Some(e) = embers.last_mut() {
                e.vy *= 1.5;
                e.heat = 1.0;
                e.size = 2;
            }
        }
    }
}

pub fn spawn_smoke(
    smoke: &mut Vec<Smoke>,
    rng: &mut LcgRng,
    fire_cx: f32,
    fire_y: f32,
    fire_w: f32,
    cols: usize,
) {
    let x = fire_cx + (rng.next_f32() - 0.5) * fire_w * 0.6;
    let vy = -0.35 - rng.next_f32() * 0.55;
    let vx = (rng.next_f32() - 0.5) * 0.45;
    let life = 2.5 + rng.next_f32() * 3.5;
    smoke.push(Smoke {
        x: x.clamp(0.0, cols.saturating_sub(1) as f32),
        y: fire_y - 2.0 - rng.next_f32() * 2.0,
        vx,
        vy,
        life,
        max_life: life,
    });
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn update_spawners(
    embers: &mut Vec<Ember>,
    smoke: &mut Vec<Smoke>,
    rng: &mut LcgRng,
    spawn_timer: &mut f32,
    smoke_timer: &mut f32,
    fire_cx: f32,
    fire_y: f32,
    fire_w: f32,
    cols: usize,
    max_embers: usize,
    max_smoke: usize,
    on_battery: bool,
    delta: f32,
) {
    *spawn_timer -= delta;
    let rate = if on_battery { 0.06 } else { 0.035 };
    while *spawn_timer <= 0.0 && embers.len() < max_embers {
        spawn_ember(embers, rng, fire_cx, fire_y, fire_w, cols);
        *spawn_timer += rate * (0.6 + rng.next_f32() * 0.8);
    }
    if *spawn_timer < 0.0 {
        *spawn_timer = 0.0;
    }

    *smoke_timer -= delta;
    let smoke_rate = if on_battery { 0.18 } else { 0.1 };
    while *smoke_timer <= 0.0 && smoke.len() < max_smoke {
        spawn_smoke(smoke, rng, fire_cx, fire_y, fire_w, cols);
        *smoke_timer += smoke_rate * (0.7 + rng.next_f32());
    }
    if *smoke_timer < 0.0 {
        *smoke_timer = 0.0;
    }
}

pub fn update_particles(
    embers: &mut Vec<Ember>,
    smoke: &mut Vec<Smoke>,
    rng: &mut LcgRng,
    cols: usize,
    delta: f32,
) {
    for e in embers.iter_mut() {
        e.life -= delta;
        e.x += e.vx * delta;
        e.y += e.vy * delta;
        e.vx += (rng.next_f32() - 0.5) * 6.0 * delta;
        e.vx *= 0.98;
        e.vy *= 0.995;
        let life_f = (e.life / e.max_life).clamp(0.0, 1.0);
        e.heat = (e.heat * 0.992).max(life_f * 0.15);
        e.heat = (e.heat - delta * 0.18).max(0.0);
    }
    embers.retain(|e| e.life > 0.0 && e.y > -1.0 && e.x > -1.0 && e.x < cols as f32 + 1.0);

    for s in smoke.iter_mut() {
        s.life -= delta;
        s.x += s.vx * delta;
        s.y += s.vy * delta;
        s.vx += (rng.next_f32() - 0.5) * 1.5 * delta;
        s.vx *= 0.99;
        s.vy *= 0.998;
    }
    smoke.retain(|s| s.life > 0.0 && s.y > -2.0 && s.x > -1.0 && s.x < cols as f32 + 1.0);
}
