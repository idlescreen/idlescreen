use crate::cosmos::Cosmos;
use crate::cosmos::types::{GravityCenter, Particle};
use crate::runner::{get_primary_monitor_bounds, is_secondary_monitor};

pub fn enter_singularity(eff: &mut Cosmos, cols: usize, rows: usize) {
    let (cx, cy) = if is_secondary_monitor() {
        (cols as f32 / 2.0, rows as f32 / 2.0)
    } else {
        let primary = get_primary_monitor_bounds(cols, rows);
        (
            (primary.start_col + primary.width() / 2) as f32,
            (primary.start_row + primary.height() / 2) as f32,
        )
    };

    let mut last_seed = None;
    for seed in &eff.seeds {
        if seed.active {
            last_seed = Some(seed);
            break;
        }
    }

    let (sx, sy, smass, scolor, svx, svy, sbirth) = if let Some(s) = last_seed {
        (
            s.x,
            s.y,
            s.mass,
            s.color,
            s.vx,
            s.vy,
            if s.is_black_hole { 10.0 } else { 0.0 },
        )
    } else {
        (cx, cy, 30.0, (130, 50, 240), 0.0, 0.0, 0.0)
    };

    eff.universe_cx = cx;
    eff.universe_cy = cy;

    eff.seeds.clear();
    eff.seeds.push(GravityCenter {
        x: sx,
        y: sy,
        vx: svx,
        vy: svy,
        mass: smass.max(30.0),
        color: scolor,
        active: true,
        is_black_hole: true,
        birth_timer: sbirth,
    });

    let flash_color = (130, 50, 240);
    for _ in 0..50 {
        let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
        let speed = eff.rng.next_range(25.0, 50.0);
        eff.particles.push(Particle {
            x: sx,
            y: sy,
            vx: svx + angle.cos() * speed,
            vy: svy + angle.sin() * speed * 0.45,
            mass: 0.5,
            color: flash_color,
            ch: '╬',
            history: Vec::new(),
        });
    }
}
