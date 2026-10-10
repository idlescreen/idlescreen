use crate::cosmos::Cosmos;
use crate::cosmos::types::Particle;

pub fn handle_seed_merges(eff: &mut Cosmos, _delta: f32, _dir: f32, seeds_len: usize) {
    let mut spawn_sparks = Vec::new();
    for i in 0..seeds_len {
        if !eff.seeds[i].active {
            continue;
        }
        for j in (i + 1)..seeds_len {
            if !eff.seeds[j].active {
                continue;
            }
            let dx = eff.seeds[j].x - eff.seeds[i].x;
            let dy = eff.seeds[j].y - eff.seeds[i].y;
            let merge_dist = 3.5 + (eff.seeds[i].mass + eff.seeds[j].mass) * 0.12;
            if dx * dx + dy * dy < merge_dist * merge_dist {
                let m_i = eff.seeds[i].mass;
                let m_j = eff.seeds[j].mass;
                let new_mass = m_i + m_j;
                eff.seeds[i].x = (eff.seeds[i].x * m_i + eff.seeds[j].x * m_j) / new_mass;
                eff.seeds[i].y = (eff.seeds[i].y * m_i + eff.seeds[j].y * m_j) / new_mass;
                eff.seeds[i].vx = (eff.seeds[i].vx * m_i + eff.seeds[j].vx * m_j) / new_mass;
                eff.seeds[i].vy = (eff.seeds[i].vy * m_i + eff.seeds[j].vy * m_j) / new_mass;
                eff.seeds[i].mass = new_mass;
                eff.seeds[j].active = false;

                let was_i_bh = eff.seeds[i].is_black_hole;
                let was_j_bh = eff.seeds[j].is_black_hole;

                let merger_type = if was_i_bh && was_j_bh {
                    3
                } else if was_i_bh || was_j_bh {
                    2
                } else {
                    i32::from(new_mass >= 8.5)
                };

                let c_i = eff.seeds[i].color;
                let c_j = eff.seeds[j].color;
                let blended_color = (
                    u16::midpoint(c_i.0 as u16, c_j.0 as u16) as u8,
                    u16::midpoint(c_i.1 as u16, c_j.1 as u16) as u8,
                    u16::midpoint(c_i.2 as u16, c_j.2 as u16) as u8,
                );

                if merger_type == 0 {
                    eff.seeds[i].color = blended_color;
                }

                spawn_sparks.push((eff.seeds[i].x, eff.seeds[i].y, merger_type, blended_color));

                if merger_type == 1 || merger_type == 2 || merger_type == 3 {
                    eff.seeds[i].is_black_hole = true;
                    eff.seeds[i].color = (130, 50, 240);
                    eff.seeds[i].birth_timer = 0.0;
                }
            }
        }
    }

    for (sx, sy, merger_type, color) in spawn_sparks {
        if merger_type == 3 {
            // Longer, softer wave than the old hard 1.2s flash.
            eff.grav_wave_max = 1.6;
            eff.grav_wave_timer = eff.grav_wave_max;
            eff.grav_wave_cx = sx;
            eff.grav_wave_cy = sy;
        }
        match merger_type {
            0 => {
                for _ in 0..25 {
                    let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                    let speed = eff.rng.next_range(15.0, 32.0);
                    eff.particles.push(Particle {
                        x: sx,
                        y: sy,
                        vx: angle.cos() * speed,
                        vy: angle.sin() * speed * 0.45,
                        mass: 0.6,
                        color: (255, 235, 180),
                        ch: if eff.rng.next_bool(0.5) { '*' } else { '+' },
                        history: Vec::new(),
                    });
                }
            }
            1 => {
                for p in &mut eff.particles {
                    let dx = p.x - sx;
                    let dy = p.y - sy;
                    let dist_sq = dx * dx + dy * dy;
                    let dist = dist_sq.sqrt().max(0.1);
                    if dist < 22.0 {
                        let push = (22.0 - dist) * 5.0;
                        p.vx += (dx / dist) * push;
                        p.vy += (dy / dist) * push * 0.45;
                    }
                }
                for _ in 0..50 {
                    let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                    let speed = eff.rng.next_range(25.0, 48.0);
                    eff.particles.push(Particle {
                        x: sx,
                        y: sy,
                        vx: angle.cos() * speed,
                        vy: angle.sin() * speed * 0.45,
                        mass: 0.8,
                        color: (255, 120, 50),
                        ch: '░',
                        history: Vec::new(),
                    });
                }
                for _ in 0..25 {
                    let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                    let speed = eff.rng.next_range(15.0, 30.0);
                    eff.particles.push(Particle {
                        x: sx,
                        y: sy,
                        vx: angle.cos() * speed,
                        vy: angle.sin() * speed * 0.45,
                        mass: 0.5,
                        color: (255, 255, 255),
                        ch: '*',
                        history: Vec::new(),
                    });
                }
            }
            2 => {
                let flare_color = (color.0.saturating_add(60), color.1.saturating_add(60), 255);
                for _ in 0..40 {
                    let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                    let speed = eff.rng.next_range(20.0, 40.0);
                    eff.particles.push(Particle {
                        x: sx,
                        y: sy,
                        vx: angle.cos() * speed,
                        vy: angle.sin() * speed * 0.45,
                        mass: 0.5,
                        color: flare_color,
                        ch: if eff.rng.next_bool(0.5) { '+' } else { '·' },
                        history: Vec::new(),
                    });
                }
            }
            3 => {
                for p in &mut eff.particles {
                    let dx = p.x - sx;
                    let dy = p.y - sy;
                    let dist_sq = dx * dx + dy * dy;
                    let dist = dist_sq.sqrt().max(0.1);
                    if dist < 32.0 {
                        let push = (32.0 - dist) * 7.5;
                        p.vx += (dx / dist) * push;
                        p.vy += (dy / dist) * push * 0.45;
                    }
                }
                for _ in 0..65 {
                    let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                    let speed = eff.rng.next_range(35.0, 65.0);
                    eff.particles.push(Particle {
                        x: sx,
                        y: sy,
                        vx: angle.cos() * speed,
                        vy: angle.sin() * speed * 0.45,
                        mass: 0.7,
                        color: (160, 80, 255),
                        ch: if eff.rng.next_bool(0.4) {
                            '╬'
                        } else if eff.rng.next_bool(0.5) {
                            '═'
                        } else {
                            '─'
                        },
                        history: Vec::new(),
                    });
                }
            }
            _ => {}
        }
    }
}
