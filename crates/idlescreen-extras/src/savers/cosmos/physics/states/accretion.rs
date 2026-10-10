use crate::cosmos::Cosmos;
use crate::cosmos::types::{GravityCenter, Particle};
use crate::runner::query_current_palette;
use crate::runner::{hsl_to_rgb, rgb_to_hsl};

pub fn enter_accretion(eff: &mut Cosmos, cols: usize) {
    let cols_f = cols as f32;
    eff.seeds.clear();
    let num_seeds = eff.rng.next_range(4.0, 10.0) as usize;
    let total_mass = 30.0;
    let mass_per_seed = total_mass / num_seeds as f32;
    let dir = if eff.spin_clockwise { 1.0f32 } else { -1.0f32 };

    let palette = query_current_palette();
    let (acc_h, _, _) = rgb_to_hsl(palette.accent.0, palette.accent.1, palette.accent.2);

    for i in 0..num_seeds {
        let angle = (i as f32 / num_seeds as f32) * std::f32::consts::TAU + eff.host_bias;
        let dist_r = eff.rng.next_range(cols_f * 0.12, cols_f * 0.38);
        let sx = eff.universe_cx + angle.cos() * dist_r;
        let sy = eff.universe_cy + angle.sin() * dist_r * 0.45;

        let tx = -angle.sin();
        let ty = angle.cos();
        let orbit_speed = (180.0 / dist_r).sqrt().clamp(3.0, 12.0);
        let vx = tx * orbit_speed * dir + eff.rng.next_range(-0.5, 0.5);
        let vy = ty * orbit_speed * 0.45 * dir + eff.rng.next_range(-0.25, 0.25);

        let seed_h = (acc_h + (i as f32 * (360.0 / num_seeds as f32))).rem_euclid(360.0);
        let seed_color = hsl_to_rgb(seed_h, 0.95, 0.55);
        eff.seeds.push(GravityCenter {
            x: sx,
            y: sy,
            vx,
            vy,
            mass: mass_per_seed,
            color: seed_color,
            active: true,
            is_black_hole: false,
            birth_timer: 0.0,
        });

        for _ in 0..15 {
            let spark_angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
            let spark_speed = eff.rng.next_range(8.0, 18.0);
            eff.particles.push(Particle {
                x: sx,
                y: sy,
                vx: vx + spark_angle.cos() * spark_speed,
                vy: vy + spark_angle.sin() * spark_speed * 0.45,
                mass: 0.4,
                color: (255, 220, 130),
                ch: '+',
                history: Vec::new(),
            });
        }
    }

    for lp in &mut eff.logo_pixels {
        lp.active = true;
    }
}
