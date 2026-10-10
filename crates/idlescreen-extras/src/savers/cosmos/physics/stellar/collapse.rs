use crate::cosmos::Cosmos;
use crate::cosmos::physics::history::push_particle_history;

pub fn update_collapse(eff: &mut Cosmos, delta: f32, cols: usize, rows: usize) {
    let (cx, cy) = if crate::runner::is_secondary_monitor() {
        (cols as f32 / 2.0, rows as f32 / 2.0)
    } else {
        let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
        (
            (primary.start_col + primary.width() / 2) as f32,
            (primary.start_row + primary.height() / 2) as f32,
        )
    };

    if !eff.seeds.is_empty() {
        let seed = &mut eff.seeds[0];
        seed.x += (cx - seed.x) * 4.0 * delta;
        seed.y += (cy - seed.y) * 4.0 * delta;
    }

    let bh_x = if eff.seeds.is_empty() {
        cx
    } else {
        eff.seeds[0].x
    };
    let bh_y = if eff.seeds.is_empty() {
        cy
    } else {
        eff.seeds[0].y
    };
    let dir = if eff.spin_clockwise { 1.0f32 } else { -1.0f32 };

    for p in &mut eff.particles {
        let dx = bh_x - p.x;
        let dy = bh_y - p.y;
        let dist = (dx * dx + dy * dy).sqrt().max(0.1);

        let pull = (220.0 + eff.state_timer * 120.0) / (dist + 1.0);
        let tangent = (60.0 - eff.state_timer * 18.0).max(0.0) / (dist.sqrt() + 1.0);

        p.vx += ((dx / dist) * pull + (dy / dist) * tangent * dir) * delta;
        p.vy += ((dy / dist) * pull * 0.45 - (dx / dist) * tangent * 0.45 * dir) * delta;

        let drag = 2.5 + eff.state_timer * 2.0;
        p.vx *= 1.0 - (delta * drag);
        p.vy *= 1.0 - (delta * drag);

        p.x += p.vx * delta;
        p.y += p.vy * delta;

        push_particle_history(&mut p.history, p.x.round() as i32, p.y.round() as i32);
    }

    eff.particles.retain(|p| {
        let dx = bh_x - p.x;
        let dy = bh_y - p.y;
        dx * dx + dy * dy > 1.5
    });
}
