use crate::cosmos::Cosmos;
use crate::cosmos::physics::{TERMINAL_ASPECT_Y, logo_to_screen_fast, to_screen_fast};
use crate::cosmos::types::Particle;

pub fn handle_logo_character_drift(
    eff: &mut Cosmos,
    delta: f32,
    dir: f32,
    _cols: usize,
    _rows: usize,
) {
    let accent = eff.cached_accent;
    let mut spawned_logo_fragments = Vec::new();

    eff.particle_screen_scratch.clear();
    eff.particle_screen_scratch.reserve(eff.particles.len());
    for p in &eff.particles {
        let (sx, sy) = to_screen_fast(
            p.x,
            p.y,
            eff.universe_cx,
            eff.universe_cy,
            eff.screen_cx,
            eff.screen_cy,
            eff.zoom,
        );
        eff.particle_screen_scratch.push((sx, sy));
    }

    for lp in &mut eff.logo_pixels {
        if !lp.active {
            continue;
        }
        lp.exc = (lp.exc - 1.2 * delta).max(0.0);

        let (lp_sx, lp_sy) = logo_to_screen_fast(
            lp.x,
            lp.y,
            eff.universe_cx,
            eff.universe_cy,
            eff.screen_cx,
            eff.screen_cy,
        );

        for &(p_sx, p_sy) in &eff.particle_screen_scratch {
            let dx = p_sx as f32 - lp_sx as f32;
            let dy = (p_sy as f32 - lp_sy as f32) * TERMINAL_ASPECT_Y;
            if dx * dx + dy * dy < 6.0 {
                lp.exc = 1.0;
                break;
            }
        }

        let mut total_bh_weight = 0.0f32;
        let mut fx_bh = 0.0f32;
        let mut fy_bh = 0.0f32;

        for seed in &eff.seeds {
            if seed.active && seed.is_black_hole {
                let (bh_sx, bh_sy) = to_screen_fast(
                    seed.x,
                    seed.y,
                    eff.universe_cx,
                    eff.universe_cy,
                    eff.screen_cx,
                    eff.screen_cy,
                    eff.zoom,
                );
                let dx = bh_sx as f32 - lp_sx as f32;
                let dy = (bh_sy as f32 - lp_sy as f32) * TERMINAL_ASPECT_Y;
                let dist_sq = dx * dx + dy * dy;
                let dist = dist_sq.sqrt().max(0.1);

                if dist < 14.0 {
                    lp.exc = 1.0;

                    if dist > 2.0 {
                        let weight = 1.0 - (dist / 14.0);
                        total_bh_weight = total_bh_weight.max(weight);

                        let pull = (seed.mass * 18.0) / (dist_sq + 6.0);
                        let tangent = (seed.mass * 12.0) / (dist.sqrt() + 2.0);
                        fx_bh += ((dx / dist) * pull + (dy / dist) * tangent * dir) * weight;
                        fy_bh += ((dy / dist) * pull - (dx / dist) * tangent * 0.45 * dir) * weight;
                    } else {
                        lp.active = false;

                        for _ in 0..10 {
                            let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
                            let speed = eff.rng.next_range(16.0, 32.0);
                            spawned_logo_fragments.push(Particle {
                                x: seed.x,
                                y: seed.y,
                                vx: angle.cos() * speed,
                                vy: angle.sin() * speed * 0.45,
                                mass: 0.5,
                                color: (
                                    (accent.0 as i16 + eff.rng.next_range(-20.0, 20.0) as i16)
                                        .clamp(0, 255) as u8,
                                    (accent.1 as i16 + eff.rng.next_range(-20.0, 20.0) as i16)
                                        .clamp(0, 255) as u8,
                                    (accent.2 as i16 + eff.rng.next_range(-20.0, 20.0) as i16)
                                        .clamp(0, 255) as u8,
                                ),
                                ch: lp.ch,
                                history: Vec::new(),
                            });
                        }
                    }
                }
            }
        }

        let dx_spring = lp.origin_x - lp.x;
        let dy_spring = lp.origin_y - lp.y;
        let k = 5.0;

        let spring_weight = 1.0 - total_bh_weight;
        let fx_spring = dx_spring * k * spring_weight;
        let fy_spring = dy_spring * k * spring_weight;

        lp.vx += (fx_spring + fx_bh) * delta;
        lp.vy += (fy_spring + fy_bh) * delta;

        let drag = 2.0;
        lp.vx *= 1.0 - (drag * delta);
        lp.vy *= 1.0 - (drag * delta);

        lp.x += lp.vx * delta;
        lp.y += lp.vy * delta;
    }
    eff.particles.extend(spawned_logo_fragments);
}
