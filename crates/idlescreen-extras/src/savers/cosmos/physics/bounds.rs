use crate::cosmos::Cosmos;
use crate::cosmos::types::Particle;

/// Playable universe extent in universe coordinates (grid edges = monitor walls).
#[derive(Clone, Copy, Debug)]
pub struct SpaceBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

impl SpaceBounds {
    pub fn for_particles(eff: &Cosmos, cols: usize, rows: usize) -> Self {
        let z = eff.zoom.max(0.2);
        Self {
            min_x: eff.universe_cx + (0.5 - eff.screen_cx) / z,
            max_x: eff.universe_cx + (cols as f32 - 0.5 - eff.screen_cx) / z,
            min_y: eff.universe_cy + (0.5 - eff.screen_cy) / z,
            max_y: eff.universe_cy + (rows as f32 - 0.5 - eff.screen_cy) / z,
        }
    }
}

fn bounce_axis(pos: &mut f32, vel: &mut f32, min: f32, max: f32, restitution: f32) {
    if *pos < min {
        *pos = min + (min - *pos) * 0.15;
        *vel = vel.abs() * restitution;
    } else if *pos > max {
        *pos = max - (*pos - max) * 0.15;
        *vel = -vel.abs() * restitution;
    }
}

pub fn clamp_particle(p: &mut Particle, bounds: SpaceBounds, restitution: f32) {
    bounce_axis(&mut p.x, &mut p.vx, bounds.min_x, bounds.max_x, restitution);
    bounce_axis(&mut p.y, &mut p.vy, bounds.min_y, bounds.max_y, restitution);
}

pub fn clamp_all_particles(eff: &mut Cosmos, cols: usize, rows: usize, restitution: f32) {
    let b = SpaceBounds::for_particles(eff, cols, rows);
    for p in &mut eff.particles {
        clamp_particle(p, b, restitution);
    }
}
