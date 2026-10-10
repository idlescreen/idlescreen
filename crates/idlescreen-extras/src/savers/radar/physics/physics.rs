// SPDX-License-Identifier: MIT

use super::collision::{
    init_jets, spawn_enemy, spawn_nuke_and_boss, spawn_scouts, update_defenders,
};
use super::jets::update_jets;
use crate::radar::types::{EnemyShip, ExplosionParticle, GoodShip, LaserBeam, RadarJet};
use crate::runner::LcgRng;
use std::time::Duration;

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn update_simulation(
    dt: Duration,
    cols: usize,
    rows: usize,
    rng: &mut LcgRng,
    next_id: &mut u32,
    sweep_angle: &mut f32,
    enemies: &mut Vec<EnemyShip>,
    defenders: &mut Vec<GoodShip>,
    jets: &mut Vec<RadarJet>,
    lasers: &mut Vec<LaserBeam>,
    particles: &mut Vec<ExplosionParticle>,
    shield_health: &mut f32,
    on_battery: bool,
    quality_scale: f32,
) {
    let dt_secs = dt.as_secs_f32();
    let bat_speed = if on_battery { 0.55 } else { 1.0 };
    let delta = dt_secs * bat_speed;
    let cx = cols as f32 / 2.0;
    let cy = rows as f32 / 2.0;

    // Density / spawn pressure thinned by battery + quality
    let spawn_scale = quality_scale * (if on_battery { 0.55 } else { 1.0 });

    // 1. Maintain 2 active radar patrol jets
    init_jets(jets, cx, cy);

    // 2. Adjust sweep angle: lock on to active nuke if present, else rotate.
    // delta already includes bat_speed, so sweep slows on battery automatically.
    let nuke_angle_opt = enemies
        .iter()
        .find(|e| e.active && e.target_type == 2)
        .map(|n| {
            let ndx = n.x - cx;
            let ndy = (n.y - cy) * 2.0;
            let mut nangle = ndy.atan2(ndx);
            if nangle < 0.0 {
                nangle += std::f32::consts::TAU;
            }
            nangle
        });

    if let Some(nangle) = nuke_angle_opt {
        *sweep_angle = nangle;
    } else {
        *sweep_angle = (*sweep_angle + delta * 1.2) % std::f32::consts::TAU;
    }

    // 3. Spawn slow moving ground batteries (fewer on battery / low quality)
    let max_batteries = ((3.0 * spawn_scale).round() as usize).max(1);
    let battery_count = enemies
        .iter()
        .filter(|e| e.active && e.target_type == 1)
        .count();
    if battery_count < max_batteries && rng.next_range(0.0, 1.0) < delta * 0.35 * spawn_scale {
        spawn_enemy(cols, rows, cx, cy, rng, next_id, enemies);
    }

    // 4. Spawn Nukes and Boss Dreadnoughts
    spawn_nuke_and_boss(
        cols,
        rows,
        cx,
        cy,
        rng,
        next_id,
        enemies,
        delta * spawn_scale,
    );

    // 5. Spawn fast scouts launched from batteries/bosses
    spawn_scouts(enemies, cx, cy, rng, next_id, delta * spawn_scale);

    // 6. Update lasers
    for laser in lasers.iter_mut() {
        laser.life -= delta;
    }
    lasers.retain(|l| l.life > 0.0);

    // 7. Update radar jets (Patrol Figure-8 OR pursue & lock enemy)
    update_jets(
        jets, enemies, lasers, particles, rng, cx, cy, cols, rows, delta,
    );

    // 8. Update enemies & detection (phosphor persistence fade)
    let mut locked_targets: Vec<u32> = jets
        .iter()
        .filter(|j| j.active)
        .filter_map(|j| j.lock_target_id)
        .collect();

    let mut reset_sector = false;

    for enemy in enemies.iter_mut().filter(|e| e.active) {
        enemy.x += enemy.vx * delta;
        enemy.y += enemy.vy * delta;
        // Phosphor persistence: soft exponential-ish decay instead of hard cut
        if enemy.visibility > 0.0 {
            enemy.visibility = (enemy.visibility - delta * 0.55).max(0.0);
        }

        if enemy.target_type == 2 {
            let ndx = cx - enemy.x;
            let ndy = cy - enemy.y;
            let ndist = (ndx * ndx + ndy * ndy).sqrt();
            let start_dist = (cols as f32 / 2.0).max(rows as f32);
            enemy.altitude = (ndist / start_dist).clamp(0.0, 1.0) * 2.5;
        }

        if enemy.x < 0.0 || enemy.x > cols as f32 || enemy.y < 0.0 || enemy.y > rows as f32 {
            enemy.active = false;
            continue;
        }

        // Shield collision check
        let edx = enemy.x - cx;
        let edy = (enemy.y - cy) * 2.0;
        let edist = (edx * edx + edy * edy).sqrt();
        if edist < 3.0 {
            enemy.active = false;
            let depletion = match enemy.target_type {
                2 => 50.0,
                3 => 40.0,
                1 => 10.0,
                _ => 5.0,
            };
            *shield_health = (*shield_health - depletion).max(0.0);
            if *shield_health <= 0.0 {
                reset_sector = true;
            }
            continue;
        }

        let dx = enemy.x - cx;
        let dy = (enemy.y - cy) * 2.0;
        let mut angle_to_enemy = dy.atan2(dx);
        if angle_to_enemy < 0.0 {
            angle_to_enemy += std::f32::consts::TAU;
        }
        let diff = (*sweep_angle - angle_to_enemy).abs();
        let wrap_diff = std::f32::consts::TAU - diff;
        let main_sweep_detect = diff < 0.15 || wrap_diff < 0.15;

        let mut jet_detect = false;
        let jet_range_sq = 8.0 * 8.0;
        for jet in jets.iter_mut().filter(|j| j.active) {
            let jdx = enemy.x - jet.x;
            let jdy = (enemy.y - jet.y) * 2.0;
            let dist_sq = jdx * jdx + jdy * jdy;

            if dist_sq <= jet_range_sq {
                let mut jet_angle_to_enemy = jdy.atan2(jdx);
                if jet_angle_to_enemy < 0.0 {
                    jet_angle_to_enemy += std::f32::consts::TAU;
                }
                let jdiff = (jet.sweep_angle - jet_angle_to_enemy).abs();
                let jwrap_diff = std::f32::consts::TAU - jdiff;
                if jdiff < 0.25 || jwrap_diff < 0.25 {
                    jet_detect = true;
                    if jet.lock_target_id.is_none() {
                        let locked = locked_targets.contains(&enemy.id);
                        if !locked && enemy.is_heavy {
                            jet.lock_target_id = Some(enemy.id);
                            jet.lock_timer = 0.0;
                            locked_targets.push(enemy.id);
                        }
                    }
                }
            }
        }

        if main_sweep_detect || jet_detect {
            // Peak phosphor charge on hit
            enemy.visibility = 3.0;

            if main_sweep_detect && enemy.is_heavy {
                let mut best_jet_idx: Option<usize> = None;
                let mut closest_dist = 9999.0;
                for (j_idx, jet) in jets.iter().enumerate() {
                    if jet.active && jet.lock_target_id.is_none() {
                        let locked = locked_targets.contains(&enemy.id);
                        if !locked {
                            let jdx = enemy.x - jet.x;
                            let jdy = enemy.y - jet.y;
                            let jdist = (jdx * jdx + jdy * jdy).sqrt();
                            if jdist < closest_dist {
                                closest_dist = jdist;
                                best_jet_idx = Some(j_idx);
                            }
                        }
                    }
                }
                if let Some(j_idx) = best_jet_idx {
                    jets[j_idx].lock_target_id = Some(enemy.id);
                    jets[j_idx].lock_timer = 0.0;
                    locked_targets.push(enemy.id);
                }
            }

            let is_tracked = defenders
                .iter()
                .any(|d| d.active && d.target_id == enemy.id);
            if !is_tracked {
                defenders.push(GoodShip {
                    x: cx,
                    y: cy,
                    target_id: enemy.id,
                    last_known_x: enemy.x,
                    last_known_y: enemy.y,
                    altitude: 0.0,
                    active: true,
                });
            }
        }
    }

    if reset_sector {
        *shield_health = 100.0;
        enemies.clear();
        defenders.clear();
    }

    // 9. Update defenders (drones) and handle anti-air shooting down
    update_defenders(defenders, enemies, rng, particles, cx, cy, delta);

    // 10. Update particles
    for p in particles.iter_mut() {
        p.x += p.vx * delta;
        p.y += p.vy * delta;
        p.life -= delta;
    }

    enemies.retain(|e| e.active);
    defenders.retain(|d| d.active);
    particles.retain(|p| p.life > 0.0);
}
