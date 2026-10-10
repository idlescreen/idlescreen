use crate::cosmos::Cosmos;
use crate::cosmos::physics::history::push_particle_history;
use crate::cosmos::types::Particle;

pub fn update_singularity(eff: &mut Cosmos, delta: f32, cols: usize, rows: usize) {
    let (cx, cy) = if crate::runner::is_secondary_monitor() {
        (cols as f32 / 2.0, rows as f32 / 2.0)
    } else {
        let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
        (
            (primary.start_col + primary.width() / 2) as f32,
            (primary.start_row + primary.height() / 2) as f32,
        )
    };
    let dir = if eff.spin_clockwise { 1.0f32 } else { -1.0f32 };

    if !eff.seeds.is_empty() {
        let seed = &mut eff.seeds[0];
        let dx_c = cx - seed.x;
        let dy_c = cy - seed.y;
        let inv_dist_c = 1.0 / (dx_c * dx_c + dy_c * dy_c).sqrt().max(0.1);

        let term_x = dx_c * inv_dist_c;
        let term_y = dy_c * inv_dist_c;
        let mut sfx = term_x * 1.8;
        let mut sfy = term_y * 0.9;

        let orbit_force = 1.8f32;
        sfx += term_y * orbit_force * dir;
        sfy -= term_x * orbit_force * 0.45 * dir;

        seed.vx += sfx * delta;
        seed.vy += sfy * delta;
        seed.vx *= 1.0 - (delta * 0.15);
        seed.vy *= 1.0 - (delta * 0.15);

        seed.x += seed.vx * delta;
        seed.y += seed.vy * delta;
    }

    let (bh_x, bh_y) = if eff.seeds.is_empty() {
        (cx, cy)
    } else {
        (eff.seeds[0].x, eff.seeds[0].y)
    };

    for p in &mut eff.particles {
        let dx = bh_x - p.x;
        let dy = bh_y - p.y;
        let dist = (dx * dx + dy * dy).sqrt().max(0.1);
        let inv_dist = 1.0 / dist;

        let pull = 110.0 / (dist + 2.0);
        let tangent = 45.0 / (dist.sqrt() + 1.0);

        let pull_factor = pull * inv_dist;
        let tangent_factor = tangent * inv_dist;

        p.vx += (dx * pull_factor + dy * tangent_factor * dir) * delta;
        p.vy += (dy * pull_factor * 0.45 - dx * tangent_factor * 0.45 * dir) * delta;

        p.vx *= 1.0 - (delta * 1.5);
        p.vy *= 1.0 - (delta * 1.5);

        p.x += p.vx * delta;
        p.y += p.vy * delta;

        push_particle_history(&mut p.history, p.x.round() as i32, p.y.round() as i32);
    }

    eff.particles.retain(|p| {
        let dx = bh_x - p.x;
        let dy = bh_y - p.y;
        dx * dx + dy * dy > 2.0
    });

    if eff.particles.len() < 200 && !eff.seeds.is_empty() && eff.rng.next_bool(0.15) {
        let seed = &eff.seeds[0];
        let dist = eff.rng.next_range(3.2, 7.5);
        let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
        let px = seed.x + angle.cos() * dist;
        let py = seed.y + angle.sin() * dist * 0.45;

        let speed = (seed.mass * 18.0 / dist).sqrt();
        let tx = -angle.sin();
        let ty = angle.cos();

        let vx = tx * speed * dir;
        let vy = ty * speed * 0.45 * dir;

        eff.particles.push(Particle {
            x: px,
            y: py,
            vx,
            vy,
            mass: eff.rng.next_range(0.4, 0.8),
            color: (160, 80, 255),
            ch: if eff.rng.next_bool(0.5) { '·' } else { '.' },
            history: Vec::new(),
        });
    }
}
