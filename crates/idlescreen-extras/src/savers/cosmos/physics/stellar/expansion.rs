use crate::cosmos::Cosmos;
use crate::cosmos::physics::bounds::clamp_all_particles;
use crate::cosmos::physics::history::push_particle_history;

pub fn update_expansion(eff: &mut Cosmos, delta: f32, cols: usize, rows: usize) {
    let dir = if eff.spin_clockwise { 1.0f32 } else { -1.0f32 };
    let progress = (eff.state_timer / 7.0).min(1.0);

    for p in &mut eff.particles {
        let dx = eff.universe_cx - p.x;
        let dy = eff.universe_cy - p.y;
        let dist = (dx * dx + dy * dy).sqrt().max(0.1);

        let pull = 10.0 * progress / (dist + 6.0);
        let tangent = 15.0 * progress / (dist.sqrt() + 2.0);

        p.vx += ((dx / dist) * pull + (dy / dist) * tangent * dir) * delta;
        p.vy += ((dy / dist) * pull * 0.45 - (dx / dist) * tangent * 0.45 * dir) * delta;

        p.vx *= 1.0 - (delta * 0.35);
        p.vy *= 1.0 - (delta * 0.35);
        p.x += p.vx * delta;
        p.y += p.vy * delta;

        push_particle_history(&mut p.history, p.x.round() as i32, p.y.round() as i32);
    }

    let progress_logo = (eff.state_timer / 8.0).min(1.0);
    let k = 0.5 + progress_logo * 5.5;
    let drag = 1.0 + progress_logo * 2.0;

    for lp in &mut eff.logo_pixels {
        if lp.active {
            let dx = lp.origin_x - lp.x;
            let dy = lp.origin_y - lp.y;

            lp.vx += dx * k * delta;
            lp.vy += dy * k * delta;

            lp.vx *= 1.0 - (drag * delta);
            lp.vy *= 1.0 - (drag * delta);

            lp.x += lp.vx * delta;
            lp.y += lp.vy * delta;

            lp.exc = (lp.exc - 0.4 * delta).max(0.0);
        }
    }

    clamp_all_particles(eff, cols, rows, 0.34);
}
