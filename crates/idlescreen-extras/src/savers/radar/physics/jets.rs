// SPDX-License-Identifier: MIT

use super::particles::{spawn_massive_explosion, spawn_nuke_explosion};
use crate::radar::types::{EnemyShip, ExplosionParticle, LaserBeam, RadarJet};
use crate::runner::LcgRng;

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn update_jets(
    jets: &mut [RadarJet],
    enemies: &mut [EnemyShip],
    lasers: &mut Vec<LaserBeam>,
    particles: &mut Vec<ExplosionParticle>,
    rng: &mut LcgRng,
    cx: f32,
    cy: f32,
    cols: usize,
    rows: usize,
    delta: f32,
) {
    for (i, jet) in jets.iter_mut().enumerate() {
        jet.t = (jet.t + delta * 0.25) % std::f32::consts::TAU;
        jet.altitude = 1.5 + 0.5 * jet.t.sin();

        let jet_range = 5.0 + jet.altitude * 2.0;
        let jet_range_sq = jet_range * jet_range;

        let mut target_x = cx;
        let mut target_y = cy;
        let mut pursuit_mode = false;

        if let Some(target_id) = jet.lock_target_id {
            if let Some(enemy) = enemies.iter().find(|e| e.active && e.id == target_id) {
                pursuit_mode = true;
                let orbit = jet.t * 2.0 + (i as f32 * std::f32::consts::FRAC_PI_2);
                target_x = enemy.x + orbit.cos() * 5.0;
                target_y = enemy.y + orbit.sin() * 2.5;

                let jdx = enemy.x - jet.x;
                let jdy = (enemy.y - jet.y) * 2.0;
                let dist_sq = jdx * jdx + jdy * jdy;
                if dist_sq <= jet_range_sq {
                    let mut jet_angle_to_enemy = jdy.atan2(jdx);
                    if jet_angle_to_enemy < 0.0 {
                        jet_angle_to_enemy += std::f32::consts::TAU;
                    }
                    let mut jdiff = jet.sweep_angle - jet_angle_to_enemy;
                    if jdiff < 0.0 {
                        jdiff += std::f32::consts::TAU;
                    }
                    if jdiff < std::f32::consts::FRAC_PI_2 {
                        jet.lock_timer += delta;
                    }
                } else {
                    jet.lock_timer = 0.0;
                }
            } else {
                jet.lock_target_id = None;
                jet.lock_timer = 0.0;
            }
        }

        if !pursuit_mode {
            let t_param = jet.t;
            target_x = cx + t_param.cos() * (cols as f32 * 0.35);
            target_y = cy + (2.0 * t_param).sin() * (rows as f32 * 0.30) * 0.5;
        }

        let dx = target_x - jet.x;
        let dy = target_y - jet.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist > 0.1 {
            let speed = if pursuit_mode { 16.0 } else { 10.0 } * delta;
            let step_x = (dx / dist) * speed;
            let step_y = (dy / dist) * speed * 0.5;
            jet.heading_ch = if step_x.abs() >= step_y.abs() {
                if step_x > 0.0 { '▶' } else { '◀' }
            } else {
                if step_y > 0.0 { '▼' } else { '▲' }
            };
            jet.x += step_x;
            jet.y += step_y;
        }

        jet.sweep_angle = (jet.sweep_angle + delta * 2.2) % std::f32::consts::TAU;

        if jet.lock_timer >= 2.0 {
            if let Some(target_id) = jet.lock_target_id
                && let Some(enemy) = enemies.iter_mut().find(|e| e.active && e.id == target_id)
            {
                lasers.push(LaserBeam {
                    from_x: cx,
                    from_y: cy,
                    to_x: enemy.x,
                    to_y: enemy.y,
                    life: 0.4,
                    max_life: 0.4,
                });
                if enemy.target_type == 3 {
                    enemy.health = enemy.health.saturating_sub(1);
                    spawn_massive_explosion(enemy.x, enemy.y, rng, particles);
                    if enemy.health == 0 {
                        enemy.active = false;
                    }
                } else if enemy.target_type == 2 {
                    enemy.active = false;
                    spawn_nuke_explosion(enemy.x, enemy.y, rng, particles);
                } else {
                    enemy.active = false;
                    spawn_massive_explosion(enemy.x, enemy.y, rng, particles);
                }
            }
            jet.lock_target_id = None;
            jet.lock_timer = 0.0;
        }
    }
}
