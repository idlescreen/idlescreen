use crate::cosmos::Cosmos;
use crate::cosmos::types::{GravityCenter, Particle};

/// Probabilistic stellar ignition — picks dense particle clumps during accretion.
pub fn handle_nebular_stellar_ignition(eff: &mut Cosmos, dir: f32) {
    if eff.state_timer <= 1.0 || eff.particles.len() <= 40 || !eff.rng.next_bool(0.10) {
        return;
    }

    let p_idx = eff.rng.next_range(0.0, eff.particles.len() as f32) as usize;
    let target_x = eff.particles[p_idx].x;
    let target_y = eff.particles[p_idx].y;

    let mut neighbors = Vec::new();
    for k in 0..eff.particles.len() {
        let dx = eff.particles[k].x - target_x;
        let dy = eff.particles[k].y - target_y;
        if dx * dx + dy * dy < 20.25 {
            neighbors.push(k);
        }
    }

    if neighbors.len() < 12 {
        return;
    }

    let mut sum_x = 0.0f32;
    let mut sum_y = 0.0f32;
    let mut sum_vx = 0.0f32;
    let mut sum_vy = 0.0f32;
    let mut sum_r = 0u32;
    let mut sum_g = 0u32;
    let mut sum_b = 0u32;

    for &idx in &neighbors {
        let p = &eff.particles[idx];
        sum_x += p.x;
        sum_y += p.y;
        sum_vx += p.vx;
        sum_vy += p.vy;
        sum_r += p.color.0 as u32;
        sum_g += p.color.1 as u32;
        sum_b += p.color.2 as u32;
    }

    let count_f = neighbors.len() as f32;
    let avg_x = sum_x / count_f;
    let avg_y = sum_y / count_f;
    let avg_vx = sum_vx / count_f;
    let avg_vy = sum_vy / count_f;
    let avg_color = (
        (sum_r / neighbors.len() as u32) as u8,
        (sum_g / neighbors.len() as u32) as u8,
        (sum_b / neighbors.len() as u32) as u8,
    );

    let dx = avg_x - eff.universe_cx;
    let dy = avg_y - eff.universe_cy;
    let dist = (dx * dx + dy * dy).sqrt().max(0.1);

    let tx = -dy / dist;
    let ty = dx / dist;

    let orbit_speed = (180.0 / dist).sqrt().clamp(4.0, 18.0);
    let orb_vx = tx * orbit_speed * dir;
    let orb_vy = ty * orbit_speed * 0.45 * dir;

    let new_vx = avg_vx * 0.2 + orb_vx * 0.8;
    let new_vy = avg_vy * 0.2 + orb_vy * 0.8;

    eff.seeds.push(GravityCenter {
        x: avg_x,
        y: avg_y,
        vx: new_vx,
        vy: new_vy,
        mass: (count_f * 0.35).clamp(1.5, 6.0),
        color: avg_color,
        active: true,
        is_black_hole: false,
        birth_timer: 0.0,
    });

    // Remove neighbor particles via descending swap_remove: processing
    // indices largest-first keeps every pending removal index valid, and
    // particle order is not load-bearing (drawn unordered). Avoids a
    // per-event O(n) retain + bool-vec allocation.
    neighbors.sort_unstable_by(|a, b| b.cmp(a));
    for idx in neighbors {
        eff.particles.swap_remove(idx);
    }

    let spark_color = (
        avg_color.0.saturating_add(80),
        avg_color.1.saturating_add(80),
        255,
    );
    for _ in 0..15 {
        let angle = eff.rng.next_range(0.0, std::f32::consts::TAU);
        let speed = eff.rng.next_range(12.0, 24.0);
        eff.particles.push(Particle {
            x: avg_x,
            y: avg_y,
            vx: avg_vx + angle.cos() * speed,
            vy: avg_vy + angle.sin() * speed * 0.45,
            mass: 0.5,
            color: spark_color,
            ch: '+',
            history: Vec::new(),
        });
    }
}
