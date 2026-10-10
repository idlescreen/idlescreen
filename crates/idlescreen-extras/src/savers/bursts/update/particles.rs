use crate::bursts::Bursts;
use crate::bursts::physics;

impl Bursts {
    pub(super) fn decay_star_excitations(&mut self, delta: f32) {
        for star in &mut self.stars {
            if star.excitation > 0.0 {
                star.excitation -= delta * 2.0;
                if star.excitation < 0.0 {
                    star.excitation = 0.0;
                }
            }
        }
    }

    pub(super) fn update_explosion_entities(&mut self, delta: f32) {
        for exp in &mut self.explosions {
            exp.x += exp.vx * delta;
            exp.y += exp.vy * delta;

            exp.vy += 28.0 * delta;
            exp.vx *= 1.0 - 0.4 * delta;
            exp.vy *= 1.0 - 0.4 * delta;

            exp.life -= delta;
        }
        self.explosions.retain(|exp| exp.life > 0.0);
    }

    pub(super) fn update_particles_and_stars(&mut self, delta: f32, cols: usize, rows: usize) {
        physics::update_particles_and_excite_stars(
            &mut self.particles,
            &mut self.stars,
            delta,
            cols,
            rows,
        );
        self.particles.retain(|p| p.life > 0.0);
    }

    /// Uneven skyline window twinkle — rooms turn on/off at different cadences.
    pub(super) fn twinkle_skyline_windows(&mut self, delta: f32) {
        if self.skyline_windows.is_empty() || self.last_cols == 0 {
            return;
        }
        let cols = self.last_cols;
        let n = self.skyline_windows.len();
        // Sparse random flips so the city feels alive without strobing.
        let flip_chance = 0.8 * delta * (if self.on_battery { 0.55 } else { 1.0 });
        for i in 0..n {
            let c = i % cols;
            let height_from_bottom = self.last_rows.saturating_sub(1).saturating_sub(i / cols);
            if height_from_bottom >= self.skyline.get(c).copied().unwrap_or(0) {
                continue;
            }
            // Host-biased phase so machines don't twinkle in lockstep.
            let phase = (i as f32 * 0.17 + self.host_bias * 6.0 + self.time_elapsed * 0.9).sin();
            if phase > 0.92 && self.rng.next_bool(flip_chance) {
                self.skyline_windows[i] = !self.skyline_windows[i];
            } else if phase < -0.95
                && self.skyline_windows[i]
                && self.rng.next_bool(flip_chance * 0.6)
            {
                self.skyline_windows[i] = false;
            }
        }
    }
}
