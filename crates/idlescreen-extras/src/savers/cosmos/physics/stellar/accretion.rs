use crate::cosmos::Cosmos;
use crate::cosmos::physics::logo_to_particle_universe;

use super::gravitation::gravitate_and_accrete_particles;
use super::ignition::handle_nebular_stellar_ignition;
use crate::cosmos::physics::drift::handle_logo_character_drift;
use crate::cosmos::physics::merges::handle_seed_merges;

pub fn update_accretion(eff: &mut Cosmos, delta: f32, cols: usize, rows: usize) {
    let dir = if eff.spin_clockwise { 1.0f32 } else { -1.0f32 };
    let seeds_len = eff.seeds.len();

    let active_logo_positions: Vec<(usize, f32, f32)> = eff
        .logo_pixels
        .iter()
        .enumerate()
        .filter(|(_, lp)| lp.active)
        .map(|(idx, lp)| {
            let (px, py) =
                logo_to_particle_universe(lp.x, lp.y, eff.universe_cx, eff.universe_cy, eff.zoom);
            (idx, px, py)
        })
        .collect();

    for i in 0..seeds_len {
        if !eff.seeds[i].active {
            continue;
        }
        let mut sfx = 0.0f32;
        let mut sfy = 0.0f32;
        for j in 0..seeds_len {
            if i == j || !eff.seeds[j].active {
                continue;
            }
            let dx = eff.seeds[j].x - eff.seeds[i].x;
            let dy = eff.seeds[j].y - eff.seeds[i].y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > 400.0 {
                continue;
            }
            let inv_dist = 1.0 / dist_sq.sqrt().max(0.1);
            let force = (eff.seeds[j].mass * 12.0) / (dist_sq + 15.0);
            let f_over_d = force * inv_dist;
            sfx += dx * f_over_d;
            sfy += dy * f_over_d;
        }
        let dx_c = eff.universe_cx - eff.seeds[i].x;
        let dy_c = eff.universe_cy - eff.seeds[i].y;
        let inv_dist_c = 1.0 / (dx_c * dx_c + dy_c * dy_c).sqrt().max(0.1);

        let orbit_force = 3.5f32 * (1.0 - (eff.state_timer * 0.06666667).min(0.85));
        let term_x = dx_c * inv_dist_c;
        let term_y = dy_c * inv_dist_c;

        sfx += term_x * 1.8;
        sfy += term_y * 0.9;
        sfx += term_y * orbit_force * dir;
        sfy -= term_x * orbit_force * 0.45 * dir;

        for &(_, lp_ux, lp_uy) in &active_logo_positions {
            let dx = lp_ux - eff.seeds[i].x;
            let dy = lp_uy - eff.seeds[i].y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > 200.0 {
                continue;
            }
            let inv_dist = 1.0 / dist_sq.sqrt().max(0.1);
            let force = 0.50 / (dist_sq + 8.0);
            let f_over_d = force * inv_dist;
            sfx += dx * f_over_d;
            sfy += dy * f_over_d;
        }

        eff.seeds[i].vx += sfx * delta;
        eff.seeds[i].vy += sfy * delta;
        eff.seeds[i].vx *= 1.0 - (delta * 0.25);
        eff.seeds[i].vy *= 1.0 - (delta * 0.25);

        eff.seeds[i].x += eff.seeds[i].vx * delta;
        eff.seeds[i].y += eff.seeds[i].vy * delta;
    }

    handle_seed_merges(eff, delta, dir, seeds_len);
    gravitate_and_accrete_particles(eff, delta, &active_logo_positions, dir);
    handle_logo_character_drift(eff, delta, dir, cols, rows);
    handle_nebular_stellar_ignition(eff, dir);
}
