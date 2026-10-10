use crate::cosmos::Cosmos;
use crate::cosmos::physics::history::push_particle_history;
use crate::cosmos::types::Particle;

pub fn gravitate_and_accrete_particles(
    eff: &mut Cosmos,
    delta: f32,
    active_logo_positions: &[(usize, f32, f32)],
    dir: f32,
) {
    eff.inv_mass_scratch.clear();
    eff.inv_mass_scratch.reserve(eff.particles.len());
    for p in &eff.particles {
        eff.inv_mass_scratch.push(1.0 / p.mass.max(1e-6));
    }
    for (p_idx, p) in eff.particles.iter_mut().enumerate() {
        let mut fx = 0.0f32;
        let mut fy = 0.0f32;
        for seed in &eff.seeds {
            if !seed.active {
                continue;
            }
            let dx = seed.x - p.x;
            let dy = seed.y - p.y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > 900.0 {
                continue;
            }
            let inv_dist = 1.0 / dist_sq.sqrt().max(0.1);

            let mass_multiplier = if seed.is_black_hole { 1.8 } else { 1.0 };
            let force = (seed.mass * 22.0 * mass_multiplier) / (dist_sq + 18.0);
            let f_over_d = force * inv_dist;
            fx += dx * f_over_d;
            fy += dy * f_over_d;
        }

        let step = if eff.quality_scale < 0.4 {
            4
        } else if eff.quality_scale < 0.7 {
            2
        } else {
            1
        };
        for &(_, lp_ux, lp_uy) in active_logo_positions.iter().step_by(step) {
            let dx = lp_ux - p.x;
            let dy = lp_uy - p.y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < 256.0 {
                let inv_dist = 1.0 / dist_sq.sqrt().max(0.1);
                let force = 0.45 / (dist_sq + 5.0);
                let f_over_d = force * inv_dist * (step as f32);
                fx += dx * f_over_d;
                fy += dy * f_over_d;
            }
        }

        let inv_mass = eff.inv_mass_scratch[p_idx];
        p.vx += (fx * delta) * inv_mass;
        p.vy += (fy * delta) * inv_mass;

        p.vx *= 1.0 - (delta * 0.40);
        p.vy *= 1.0 - (delta * 0.40);

        p.x += p.vx * delta;
        p.y += p.vy * delta;

        push_particle_history(&mut p.history, p.x.round() as i32, p.y.round() as i32);
    }

    // Accrete particles
    let mut new_sparks = Vec::new();
    let current_total_particles = eff.particles.len();
    let accent = eff.cached_accent;
    eff.particles.retain_mut(|p| {
        for &(_, lp_ux, lp_uy) in active_logo_positions {
            let dx = lp_ux - p.x;
            let dy = lp_uy - p.y;
            if dx * dx + dy * dy < 1.44 {
                if current_total_particles + new_sparks.len() < (400.0 * eff.quality_scale) as usize
                {
                    let ox = p.x - eff.universe_cx;
                    let oy = p.y - eff.universe_cy;
                    let inv_o_len = 1.0 / (ox * ox + oy * oy).sqrt().max(0.1);
                    let dir_x = ox * inv_o_len;
                    let dir_y = oy * inv_o_len;

                    let spark_count = eff.rng.next_range(2.0, 4.0) as usize;
                    for _ in 0..spark_count {
                        let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                        let speed = eff.rng.next_range(50.0, 95.0);
                        new_sparks.push(Particle {
                            x: p.x,
                            y: p.y,
                            vx: (dir_x * 0.75 + angle.cos() * 0.25) * speed,
                            vy: (dir_y * 0.75 + angle.sin() * 0.25) * speed * 0.45,
                            mass: 0.5,
                            color: (
                                accent.0.saturating_add(60),
                                accent.1.saturating_add(60),
                                255,
                            ),
                            ch: if eff.rng.next_bool(0.5) { '*' } else { '+' },
                            history: Vec::new(),
                        });
                    }
                }
                return false;
            }
        }

        for seed in &mut eff.seeds {
            if seed.active {
                let dx = seed.x - p.x;
                let dy = seed.y - p.y;
                let dist_sq = dx * dx + dy * dy;
                if seed.is_black_hole {
                    if dist_sq < 2.25 {
                        if current_total_particles + new_sparks.len()
                            < (350.0 * eff.quality_scale) as usize
                        {
                            let spark_count = eff.rng.next_range(2.0, 4.0) as usize;
                            for _ in 0..spark_count {
                                let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                                let speed = eff.rng.next_range(12.0, 24.0);
                                new_sparks.push(Particle {
                                    x: seed.x + angle.cos() * 1.6,
                                    y: seed.y + angle.sin() * 1.6 * 0.45,
                                    vx: angle.cos() * speed,
                                    vy: angle.sin() * speed * 0.45,
                                    mass: 0.5,
                                    color: (180, 100, 255),
                                    ch: if eff.rng.next_bool(0.5) { '+' } else { '·' },
                                    history: Vec::new(),
                                });
                            }
                        }
                        return false;
                    }
                } else if dist_sq < 1.44 {
                    seed.mass += 0.08;
                    if eff.rng.next_bool(0.4)
                        && current_total_particles + new_sparks.len()
                            < (350.0 * eff.quality_scale) as usize
                    {
                        let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                        let speed = eff.rng.next_range(8.0, 18.0);
                        new_sparks.push(Particle {
                            x: seed.x + angle.cos() * 1.3,
                            y: seed.y + angle.sin() * 1.3 * 0.45,
                            vx: angle.cos() * speed,
                            vy: angle.sin() * speed * 0.45,
                            mass: 0.4,
                            color: (255, 230, 150),
                            ch: '·',
                            history: Vec::new(),
                        });
                    }
                    return false;
                }
            }
        }
        true
    });
    eff.particles.extend(new_sparks);

    // Spawn orbital replenishment particles around black holes
    if eff.particles.len() < (250.0 * eff.quality_scale) as usize {
        for seed in &eff.seeds {
            if seed.active && seed.is_black_hole && eff.rng.next_bool(0.12) {
                let dist = eff.rng.next_range(3.2, 7.5);
                let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                let px = seed.x + angle.cos() * dist;
                let py = seed.y + angle.sin() * dist * 0.45;

                let speed = (seed.mass * 12.0 / dist).sqrt();
                let tx = -angle.sin();
                let ty = angle.cos();

                let vx = tx * speed * dir;
                let vy = ty * speed * 0.45 * dir;

                let p_color = (
                    (seed.color.0 as i16 + eff.rng.next_range(-20.0, 20.0) as i16).clamp(0, 255)
                        as u8,
                    (seed.color.1 as i16 + eff.rng.next_range(-20.0, 20.0) as i16).clamp(0, 255)
                        as u8,
                    (seed.color.2 as i16 + eff.rng.next_range(-20.0, 20.0) as i16).clamp(0, 255)
                        as u8,
                );
                eff.particles.push(Particle {
                    x: px,
                    y: py,
                    vx,
                    vy,
                    mass: eff.rng.next_range(0.4, 0.8),
                    color: p_color,
                    ch: if eff.rng.next_bool(0.5) { '·' } else { '.' },
                    history: Vec::new(),
                });
            }
        }
    }
}
