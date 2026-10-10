// SPDX-License-Identifier: MIT

use super::particles::{spawn_explosion, spawn_failure_puff};
use crate::radar::types::{EnemyShip, ExplosionParticle, GoodShip, RadarJet};
use crate::runner::LcgRng;

fn calc_velocity(ex: f32, ey: f32, cx: f32, cy: f32, speed: f32) -> (f32, f32) {
    let dx = cx - ex;
    let dy = cy - ey;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist > 0.1 {
        ((dx / dist) * speed, (dy / dist) * speed)
    } else {
        (0.0, 0.0)
    }
}

pub fn update_defenders(
    defenders: &mut [GoodShip],
    enemies: &mut [EnemyShip],
    rng: &mut LcgRng,
    particles: &mut Vec<ExplosionParticle>,
    cx: f32,
    cy: f32,
    delta: f32,
) {
    let seeker_range = 12.0;
    let anti_air_range_sq = 6.0 * 6.0;

    for def in defenders.iter_mut().filter(|d| d.active) {
        let mut target_x = def.last_known_x;
        let mut target_y = def.last_known_y;
        let mut has_seeker_lock = false;

        if let Some(enemy) = enemies.iter().find(|e| e.active && e.id == def.target_id) {
            let dx = enemy.x - def.x;
            let dy = enemy.y - def.y;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq <= anti_air_range_sq
                && (enemy.target_type == 3
                    || (enemy.target_type == 1 && rng.next_range(0.0, 1.0) < 0.90))
            {
                def.active = false;
                spawn_failure_puff(def.x, def.y, rng, particles);
                continue;
            }

            if dist_sq.sqrt() <= seeker_range {
                target_x = enemy.x;
                target_y = enemy.y;
                has_seeker_lock = true;
            }
        }

        let dx = target_x - def.x;
        let dy = target_y - def.y;
        let dist = (dx * dx + dy * dy).sqrt();

        // Calculate drone parabolic altitude
        let dx_base = def.x - cx;
        let dy_base = def.y - cy;
        let d_base = (dx_base * dx_base + dy_base * dy_base).sqrt();
        let total = d_base + dist;
        if total > 0.1 {
            let progress = d_base / total;
            def.altitude = 1.2 * (4.0 * progress * (1.0 - progress));
        } else {
            def.altitude = 0.0;
        }

        if dist > 0.5 {
            let speed = 14.0 * delta;
            def.x += (dx / dist) * speed;
            def.y += (dy / dist) * speed * 0.5;
        } else {
            def.active = false;
            spawn_failure_puff(def.x, def.y, rng, particles);
            continue;
        }

        if has_seeker_lock && dist < 1.5 {
            def.active = false;
            if let Some(enemy) = enemies
                .iter_mut()
                .find(|e| e.active && e.id == def.target_id)
            {
                if enemy.target_type == 3 {
                    spawn_failure_puff(def.x, def.y, rng, particles);
                } else {
                    enemy.active = false;
                    spawn_explosion(enemy.x, enemy.y, rng, particles);
                }
            }
        }
    }
}

pub fn spawn_enemy(
    cols: usize,
    rows: usize,
    cx: f32,
    cy: f32,
    rng: &mut LcgRng,
    next_id: &mut u32,
    enemies: &mut Vec<EnemyShip>,
) {
    let edge = rng.next_range(0.0, 4.0) as u32;
    let (ex, ey) = match edge {
        0 => (rng.next_range(0.0, cols as f32), 0.0f32),
        1 => (rng.next_range(0.0, cols as f32), rows as f32),
        2 => (0.0f32, rng.next_range(0.0, rows as f32)),
        _ => (cols as f32, rng.next_range(0.0, rows as f32)),
    };
    let (vx, vy) = calc_velocity(ex, ey, cx, cy, 0.5);
    enemies.push(EnemyShip {
        id: *next_id,
        x: ex,
        y: ey,
        vx,
        vy,
        visibility: 0.0,
        is_heavy: true,
        target_type: 1,
        health: 1,
        altitude: 0.0,
        active: true,
    });
    *next_id = next_id.wrapping_add(1);
}

pub fn init_jets(jets: &mut Vec<RadarJet>, cx: f32, cy: f32) {
    if jets.len() < 2 {
        while jets.len() < 2 {
            let id = jets.len();
            jets.push(RadarJet {
                x: cx,
                y: cy,
                heading_ch: '▶',
                sweep_angle: 0.0,
                t: if id == 0 { 0.0 } else { std::f32::consts::PI },
                lock_target_id: None,
                lock_timer: 0.0,
                altitude: 1.5,
                active: true,
            });
        }
    }
}

pub fn spawn_scouts(
    enemies: &mut Vec<EnemyShip>,
    cx: f32,
    cy: f32,
    rng: &mut LcgRng,
    next_id: &mut u32,
    delta: f32,
) {
    let mut new_scouts = Vec::new();
    for enemy in enemies
        .iter()
        .filter(|e| e.active && (e.target_type == 1 || e.target_type == 3))
    {
        let spawn_rate = if enemy.target_type == 3 { 0.35 } else { 0.22 };
        if rng.next_range(0.0, 1.0) < delta * spawn_rate {
            let (vx, vy) = calc_velocity(enemy.x, enemy.y, cx, cy, 1.6);
            new_scouts.push(EnemyShip {
                id: *next_id,
                x: enemy.x,
                y: enemy.y,
                vx,
                vy,
                visibility: 3.0,
                is_heavy: false,
                target_type: 0,
                health: 1,
                altitude: 0.0,
                active: true,
            });
            *next_id = next_id.wrapping_add(1);
        }
    }
    enemies.extend(new_scouts);
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn spawn_nuke_and_boss(
    cols: usize,
    rows: usize,
    cx: f32,
    cy: f32,
    rng: &mut LcgRng,
    next_id: &mut u32,
    enemies: &mut Vec<EnemyShip>,
    delta: f32,
) {
    // 1. Spawn Tactical Nuke (type 2)
    let nuke_active = enemies.iter().any(|e| e.active && e.target_type == 2);
    if !nuke_active && rng.next_range(0.0, 1.0) < delta * 0.08 {
        let ex = rng.next_range(10.0, cols as f32 - 10.0);
        let ey = 0.0f32;
        let (vx, vy) = calc_velocity(ex, ey, cx, cy, 1.8);
        enemies.push(EnemyShip {
            id: *next_id,
            x: ex,
            y: ey,
            vx,
            vy,
            visibility: 3.0,
            is_heavy: true,
            target_type: 2,
            health: 1,
            altitude: 2.5,
            active: true,
        });
        *next_id = next_id.wrapping_add(1);
    }

    // 2. Spawn Boss Dreadnought (type 3)
    let boss_active = enemies.iter().any(|e| e.active && e.target_type == 3);
    if !boss_active && rng.next_range(0.0, 1.0) < delta * 0.03 {
        let edge = rng.next_range(0.0, 4.0) as u32;
        let (ex, ey) = match edge {
            0 => (rng.next_range(0.0, cols as f32), 0.0f32),
            1 => (rng.next_range(0.0, cols as f32), rows as f32),
            2 => (0.0f32, rng.next_range(0.0, rows as f32)),
            _ => (cols as f32, rng.next_range(0.0, rows as f32)),
        };
        let (vx, vy) = calc_velocity(ex, ey, cx, cy, 0.3);
        enemies.push(EnemyShip {
            id: *next_id,
            x: ex,
            y: ey,
            vx,
            vy,
            visibility: 3.0,
            is_heavy: true,
            target_type: 3,
            health: 3,
            altitude: 0.0,
            active: true,
        });
        *next_id = next_id.wrapping_add(1);
    }
}
