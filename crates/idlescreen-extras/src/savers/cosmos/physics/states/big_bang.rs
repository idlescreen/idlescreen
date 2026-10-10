use crate::cosmos::Cosmos;
use crate::cosmos::types::Particle;
use crate::runner::{get_primary_monitor_bounds, is_secondary_monitor, query_current_palette};
use crate::runner::{hsl_to_rgb, rgb_to_hsl};

pub fn enter_big_bang(eff: &mut Cosmos, cols: usize, rows: usize) {
    let (cx, cy) = if is_secondary_monitor() {
        (cols as f32 / 2.0, rows as f32 / 2.0)
    } else {
        let primary = get_primary_monitor_bounds(cols, rows);
        (
            (primary.start_col + primary.width() / 2) as f32,
            (primary.start_row + primary.height() / 2) as f32,
        )
    };

    eff.particles.clear();
    eff.seeds.clear();

    eff.spin_clockwise = eff.rng.next_bool(0.5);
    let dir = if eff.spin_clockwise { 1.0f32 } else { -1.0f32 };
    eff.universe_cx = cx;
    eff.universe_cy = cy;

    let palette = query_current_palette();
    let (acc_h, _, _) = rgb_to_hsl(palette.accent.0, palette.accent.1, palette.accent.2);

    for lp in &mut eff.logo_pixels {
        lp.active = true;
        lp.dissolved = false;
        lp.shards_pending = 0;
        let rx = eff.rng.next_range(-2.5, 2.5);
        let ry = eff.rng.next_range(-1.2, 1.2);
        lp.x = eff.universe_cx + rx;
        lp.y = eff.universe_cy + ry;
        let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
        let speed = eff.rng.next_range(20.0, 70.0);
        let swirl_speed = speed * 0.22;
        lp.vx = angle.cos() * speed - angle.sin() * swirl_speed * dir;
        lp.vy = angle.sin() * speed * 0.42 + angle.cos() * swirl_speed * 0.42 * dir;
        lp.exc = 1.0;
    }

    let mut num_particles = 120 + (eff.seed_density_opt * 45).min(350) as usize;
    if eff.on_battery {
        num_particles = (num_particles as f32 * 0.55) as usize;
    }
    num_particles = (num_particles as f32 * eff.quality_scale) as usize;

    for _ in 0..(num_particles * 2 / 3) {
        let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
        let speed = eff.rng.next_range(15.0, 55.0);
        let swirl_speed = speed * 0.20;
        let vx = angle.cos() * speed - angle.sin() * swirl_speed * dir;
        let vy = angle.sin() * speed * 0.42 + angle.cos() * swirl_speed * 0.42 * dir;

        let p_hue = (acc_h + eff.rng.next_range(-40.0, 40.0)).rem_euclid(360.0);
        let color = hsl_to_rgb(p_hue, 0.90, 0.55);

        let rx = eff.rng.next_range(-3.5, 3.5);
        let ry = eff.rng.next_range(-1.6, 1.6);

        eff.particles.push(Particle {
            x: eff.universe_cx + rx,
            y: eff.universe_cy + ry,
            vx,
            vy,
            mass: eff.rng.next_range(0.7, 1.3),
            color,
            ch: if eff.rng.next_bool(0.4) {
                '+'
            } else if eff.rng.next_bool(0.5) {
                '·'
            } else {
                '.'
            },
            history: Vec::new(),
        });
    }

    for _ in 0..(num_particles / 3) {
        let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
        let speed = eff.rng.next_range(65.0, 95.0);
        let swirl_speed = speed * 0.15;
        let vx = angle.cos() * speed - angle.sin() * swirl_speed * dir;
        let vy = angle.sin() * speed * 0.42 + angle.cos() * swirl_speed * 0.42 * dir;

        let color = if eff.rng.next_bool(0.5) {
            (255, 220, 100)
        } else {
            (100, 240, 255)
        };

        let rx = eff.rng.next_range(-2.0, 2.0);
        let ry = eff.rng.next_range(-0.9, 0.9);

        eff.particles.push(Particle {
            x: eff.universe_cx + rx,
            y: eff.universe_cy + ry,
            vx,
            vy,
            mass: eff.rng.next_range(0.5, 0.8),
            color,
            ch: '*',
            history: Vec::new(),
        });
    }
}
