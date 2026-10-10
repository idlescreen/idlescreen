use crate::cosmos::Cosmos;
use crate::cosmos::physics::default_zoom;
use crate::cosmos::types::Particle;
use crate::runner::{get_primary_monitor_bounds, is_secondary_monitor};

pub fn enter_darkness(eff: &mut Cosmos, cols: usize, rows: usize) {
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
    eff.universe_cx = cx;
    eff.universe_cy = cy;
    eff.zoom = default_zoom(cols, rows);

    for _ in 0..20 {
        let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
        let speed = eff.rng.next_range(2.0, 8.0);
        eff.particles.push(Particle {
            x: eff.universe_cx,
            y: eff.universe_cy,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed * 0.45,
            mass: 0.3,
            color: (100, 100, 100),
            ch: '·',
            history: Vec::new(),
        });
    }
}
