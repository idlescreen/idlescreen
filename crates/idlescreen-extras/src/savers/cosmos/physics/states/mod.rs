mod accretion;
mod big_bang;
mod darkness;
mod singularity;

use crate::cosmos::Cosmos;
use crate::cosmos::types::{Particle, UniverseState};

pub fn enter_state(eff: &mut Cosmos, cols: usize, rows: usize) {
    match eff.state {
        UniverseState::Darkness => darkness::enter_darkness(eff, cols, rows),
        UniverseState::BigBang => big_bang::enter_big_bang(eff, cols, rows),
        UniverseState::Expansion => enter_expansion(eff),
        UniverseState::Accretion => accretion::enter_accretion(eff, cols),
        UniverseState::Singularity => singularity::enter_singularity(eff, cols, rows),
        UniverseState::Collapse => enter_collapse(eff),
    }
}

fn enter_expansion(eff: &mut Cosmos) {
    for lp in &mut eff.logo_pixels {
        lp.active = true;
    }
}

fn enter_collapse(eff: &mut Cosmos) {
    let bh_x = eff.universe_cx;
    let bh_y = eff.universe_cy;
    for i in 0..50 {
        let angle = (i as f32 / 50.0) * std::f32::consts::TAU;
        let dist = 35.0f32;
        let px = bh_x + angle.cos() * dist;
        let py = bh_y + angle.sin() * dist * 0.45;
        let speed = -45.0f32;
        eff.particles.push(Particle {
            x: px,
            y: py,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed * 0.45,
            mass: 0.6,
            color: (255, 255, 255),
            ch: '░',
            history: Vec::new(),
        });
    }
}
