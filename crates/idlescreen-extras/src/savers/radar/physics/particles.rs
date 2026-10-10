// SPDX-License-Identifier: MIT

use crate::radar::types::ExplosionParticle;
use crate::runner::LcgRng;

pub fn spawn_explosion(x: f32, y: f32, rng: &mut LcgRng, particles: &mut Vec<ExplosionParticle>) {
    for _ in 0..10 {
        let angle = rng.next_range(0.0, std::f32::consts::TAU);
        let speed = rng.next_range(3.0, 7.0);
        particles.push(ExplosionParticle {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed * 0.5,
            life: 0.8,
            max_life: rng.next_range(0.5, 1.0),
        });
    }
}

pub fn spawn_massive_explosion(
    x: f32,
    y: f32,
    rng: &mut LcgRng,
    particles: &mut Vec<ExplosionParticle>,
) {
    for _ in 0..30 {
        let angle = rng.next_range(0.0, std::f32::consts::TAU);
        let speed = rng.next_range(6.0, 15.0);
        particles.push(ExplosionParticle {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed * 0.5,
            life: 1.5,
            max_life: rng.next_range(0.8, 1.6),
        });
    }
}

pub fn spawn_failure_puff(
    x: f32,
    y: f32,
    rng: &mut LcgRng,
    particles: &mut Vec<ExplosionParticle>,
) {
    for _ in 0..6 {
        let angle = rng.next_range(0.0, std::f32::consts::TAU);
        let speed = rng.next_range(1.5, 3.5);
        particles.push(ExplosionParticle {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed * 0.5,
            life: 0.6,
            max_life: rng.next_range(0.4, 0.7),
        });
    }
}

pub fn spawn_nuke_explosion(
    x: f32,
    y: f32,
    rng: &mut LcgRng,
    particles: &mut Vec<ExplosionParticle>,
) {
    for _ in 0..80 {
        let angle = rng.next_range(0.0, std::f32::consts::TAU);
        let speed = rng.next_range(8.0, 24.0);
        particles.push(ExplosionParticle {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed * 0.5,
            life: 2.2,
            max_life: rng.next_range(1.5, 2.5),
        });
    }
}
