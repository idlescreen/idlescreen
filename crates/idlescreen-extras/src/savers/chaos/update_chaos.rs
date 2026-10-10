use super::{Chaos, ExplosionType, Phase};

impl Chaos {
    pub fn update_chaos(&mut self, delta: f32, cols: usize, rows: usize) {
        let center_x = self.center_x;
        let center_y = self.center_y;

        // Handle Black Hole secondary burst
        let burst_time = match self.explosion_freq_opt {
            0 => 2.8,
            2 => 0.7,
            _ => 1.4,
        };
        if self.explosion_type == ExplosionType::BlackHole
            && self.phase_timer >= burst_time
            && !self.black_hole_burst_triggered
        {
            self.black_hole_burst_triggered = true;
            let speed_scale = crate::runner::span_reach_scale(cols, rows).sqrt();

            for p in &mut self.particles {
                let mut dx = p.x - center_x;
                let mut dy = (p.y - center_y) * 2.2;
                if dx.abs() < 0.1 && dy.abs() < 0.1 {
                    dx = self.rng.next_range(-1.0, 1.0);
                    dy = self.rng.next_range(-1.0, 1.0);
                }
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                let speed = self.rng.next_range(35.0, 65.0) * speed_scale;
                p.vx = (dx / dist) * speed + self.rng.next_range(-4.0, 4.0);
                p.vy = ((dy / dist) * speed + self.rng.next_range(-4.0, 4.0)) * 0.48;
                p.glow = 1.5;
            }
        }

        for p in &mut self.particles {
            match self.explosion_type {
                ExplosionType::Supernova => {
                    p.x += p.vx * delta;
                    p.y += p.vy * delta;
                    p.vx *= 1.0 - 0.85 * delta;
                    p.vy *= 1.0 - 0.85 * delta;
                }
                ExplosionType::BlackHole => {
                    if self.black_hole_burst_triggered {
                        p.vx *= 1.0 - 0.7 * delta;
                        p.vy *= 1.0 - 0.7 * delta;
                    } else {
                        let dx = center_x - p.x;
                        let dy = (center_y - p.y) * 2.2;
                        let dist = (dx * dx + dy * dy).sqrt().max(0.5);

                        let pull = 160.0 / dist.max(2.0);
                        let angle = dy.atan2(dx);
                        let spin = 35.0 / dist.max(2.0);

                        p.vx += (pull * angle.cos() - spin * angle.sin()) * delta;
                        p.vy += (pull * angle.sin() + spin * angle.cos()) * delta * 0.48;

                        p.vx *= 1.0 - 0.9 * delta;
                        p.vy *= 1.0 - 0.9 * delta;
                    }
                    p.x += p.vx * delta;
                    p.y += p.vy * delta;
                }
                ExplosionType::Vortex => {
                    let dx = p.x - center_x;
                    let dy = (p.y - center_y) * 2.2;

                    let angle = dy.atan2(dx);
                    let pull_strength = -6.0;
                    let spin_strength = 28.0;

                    p.vx += (pull_strength * angle.cos() - spin_strength * angle.sin()) * delta;
                    p.vy +=
                        (pull_strength * angle.sin() + spin_strength * angle.cos()) * delta * 0.48;

                    p.vx *= 1.0 - 0.4 * delta;
                    p.vy *= 1.0 - 0.4 * delta;

                    p.x += p.vx * delta;
                    p.y += p.vy * delta;
                }
                ExplosionType::GlitchWave => {
                    let wave = (self.phase_timer * 18.0 + p.y * 0.4).sin() * 22.0;
                    p.vx += wave * delta;
                    p.vy += self.rng.next_range(-6.0, 6.0) * delta;

                    p.vx *= 1.0 - 1.4 * delta;
                    p.vy *= 1.0 - 1.4 * delta;

                    p.x += p.vx * delta;
                    p.y += p.vy * delta;

                    if self.rng.next_bool(0.08) {
                        let glitch_chars =
                            ['1', '0', '█', '░', '▒', '▞', '*', '$', '#', '@', '&', '%'];
                        p.ch = glitch_chars[self.rng.next_usize(glitch_chars.len())];
                    } else if self.rng.next_bool(0.1) {
                        p.ch = p.orig_ch;
                    }
                }
                ExplosionType::Shockwave => {
                    if self.rng.next_bool(0.06) {
                        p.ch = if self.rng.next_bool(0.5) {
                            '═'
                        } else {
                            '║'
                        };
                    }

                    let dx = p.x - center_x;
                    let dy = (p.y - center_y) * 2.2;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                    let angle = dy.atan2(dx);

                    let wave = (self.phase_timer * 12.0 + dist * 0.1).sin() * 8.0;
                    let push = 18.0 / dist.max(4.0);

                    p.vx += (push * angle.cos() + wave * 0.3) * delta;
                    p.vy += (push * angle.sin() * 0.48) * delta;

                    p.vx *= 1.0 - 0.78 * delta;
                    p.vy *= 1.0 - 0.78 * delta;

                    p.x += p.vx * delta;
                    p.y += p.vy * delta;
                }
                ExplosionType::Entropy => {
                    let jitter = 14.0 + (self.phase_timer * 6.0).min(35.0);
                    p.vx += self.rng.next_range(-jitter, jitter) * delta;
                    p.vy += self.rng.next_range(-jitter * 0.4, jitter * 0.4) * delta;

                    let dx = p.x - center_x;
                    let dy = (p.y - center_y) * 2.2;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                    let push = 3.0 / dist.max(3.0);
                    let angle = dy.atan2(dx);

                    p.vx += push * angle.cos() * delta;
                    p.vy += push * angle.sin() * 0.48 * delta;

                    p.vx *= 1.0 - 0.65 * delta;
                    p.vy *= 1.0 - 0.65 * delta;

                    p.x += p.vx * delta;
                    p.y += p.vy * delta;

                    if self.rng.next_bool(0.12) {
                        let corrupt_chars = ['#', '@', '%', '&', '░', '▒', '▓', '█', '0', '1'];
                        p.ch = corrupt_chars[self.rng.next_usize(corrupt_chars.len())];
                    }
                }
                ExplosionType::Resonance => {
                    let dx = p.x - center_x;
                    let dy = (p.y - center_y) * 2.2;
                    let angle = dy.atan2(dx);

                    let oscillation = (self.phase_timer * 22.0).sin() * 18.0;
                    let base_pull = 5.0;

                    p.vx += (base_pull * angle.cos() + oscillation * angle.cos() * 0.6) * delta;
                    p.vy += (base_pull * angle.sin() * 0.48
                        + oscillation * angle.sin() * 0.48 * 0.6)
                        * delta;

                    p.vx *= 1.0 - 0.55 * delta;
                    p.vy *= 1.0 - 0.55 * delta;

                    p.x += p.vx * delta;
                    p.y += p.vy * delta;
                }
            }

            let bounce_loss = 0.72;
            if p.x < 0.0 {
                p.x = 0.0;
                p.vx = -p.vx * bounce_loss;
            } else if p.x >= cols as f32 {
                p.x = cols.saturating_sub(1) as f32;
                p.vx = -p.vx * bounce_loss;
            }

            if p.y < 0.0 {
                p.y = 0.0;
                p.vy = -p.vy * bounce_loss;
            } else if p.y >= rows as f32 {
                p.y = rows.saturating_sub(1) as f32;
                p.vy = -p.vy * bounce_loss;
            }

            if p.glow > 0.1 {
                p.glow -= delta * 0.15;
            }
        }

        // Slightly shorter chaos on battery so the readable logo returns sooner.
        let bat = if self.on_battery { 0.8 } else { 1.0 };
        let limit = match self.explosion_freq_opt {
            0 => 10.0,
            2 => 2.5,
            _ => 5.0,
        } * bat;
        if self.phase_timer > limit {
            self.phase = Phase::SnapBack;
            self.phase_timer = 0.0;
        }
    }
}
