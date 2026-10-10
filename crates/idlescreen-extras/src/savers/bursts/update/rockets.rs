use crate::bursts::Bursts;
use crate::bursts::types::{Explosion, FIREWORK_COLORS, Particle, Rocket, Star};

impl Bursts {
    pub(super) fn refresh_live_stats(&mut self, delta: f32) {
        self.sys_refresh_timer += delta;
        if self.sys_refresh_timer >= 1.0 {
            let sys = crate::runner::get_system_info();
            self.mem_pressure = sys.mem_used_pct / 100.0;
            self.cpu_load = (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
            self.on_battery = sys.power_status.contains("Battery");
            self.target_accent = crate::runner::query_current_palette().accent;
            self.logo_text = sys.logo_text.clone();
            self.sys_refresh_timer = 0.0;
        }

        // Smooth 1-second RGB lerp toward target accent
        let t = (delta / 1.0).clamp(0.0, 1.0);
        self.current_accent.0 += (self.target_accent.0 as f32 - self.current_accent.0) * t;
        self.current_accent.1 += (self.target_accent.1 as f32 - self.current_accent.1) * t;
        self.current_accent.2 += (self.target_accent.2 as f32 - self.current_accent.2) * t;
        self.accent = (
            self.current_accent.0.round() as u8,
            self.current_accent.1.round() as u8,
            self.current_accent.2.round() as u8,
        );
    }

    pub(super) fn resize_if_needed(&mut self, cols: usize, rows: usize) {
        if cols != self.last_cols || rows != self.last_rows {
            self.generate_skyline(cols, rows);
            self.rockets.clear();
            self.particles.clear();
            self.explosions.clear();
            self.stars.clear();
            self.last_cols = cols;
            self.last_rows = rows;
            self.intro_fade = 0.0;
            self.launch_cooldown = 0.35 + self.host_bias * 0.4;
            self.quiet_timer = 0.0;
            self.wave_launches = 0;
        }
    }

    pub(super) fn sync_star_population(&mut self, cols: usize, rows: usize) {
        let bat = if self.on_battery { 0.55 } else { 1.0 };
        let target_stars =
            (((cols * rows / 20).clamp(10, 80)) as f32 * self.quality_scale * bat) as usize;
        if self.stars.len() > target_stars {
            self.stars.truncate(target_stars);
        } else if self.stars.len() < target_stars && target_stars > 0 {
            while self.stars.len() < target_stars {
                self.stars.push(Star {
                    x: self.rng.next_f32(),
                    y: self.rng.next_f32(),
                    phase: self.rng.next_f32() * std::f32::consts::TAU,
                    ch: if self.stars.len().is_multiple_of(8) {
                        '✦'
                    } else if self.stars.len().is_multiple_of(3) {
                        '+'
                    } else {
                        '.'
                    },
                    excitation: 0.0,
                    excited_color: (255, 255, 255),
                });
            }
        }
    }

    pub(super) fn maybe_launch_rocket(&mut self, cols: usize, rows: usize) {
        let audio = idle_api::audio::query_audio_bands();
        let audio_peak = audio[0].max(audio[1] * 0.9);
        let audio_boost = audio_peak > 0.45;
        if audio_boost {
            self.quiet_timer = 0.0;
        } else if self.quiet_timer > 0.0 || self.launch_cooldown > 0.0 {
            return;
        }

        let load_mult = 1.0 + self.cpu_load * 0.7 + self.mem_pressure * 0.3;
        let bat = if self.on_battery { 0.55 } else { 1.0 };
        let (base_max, base_chance) = match self.launch_rate_opt {
            0 => (2, 0.015),
            2 => (5, 0.06),
            _ => (3, 0.035),
        };
        // Cap concurrent rockets lower so we never dump a full volley at once.
        let max_rockets =
            (base_max as f32 * load_mult * self.quality_scale * bat).clamp(1.0, 4.0) as usize;
        let chance = if audio_boost {
            (base_chance * 4.0_f32).min(0.85_f32)
        } else {
            base_chance * load_mult * self.quality_scale * bat
        };
        if self.rockets.len() < max_rockets && self.rng.next_bool(chance) {
            let bounds = if crate::runner::is_secondary_monitor() {
                crate::runner::MonitorCellBounds {
                    start_col: 0,
                    end_col: cols,
                    start_row: 0,
                    end_row: rows,
                    is_primary: false,
                }
            } else {
                crate::runner::get_primary_monitor_bounds(cols, rows)
            };
            let start_x = self
                .rng
                .next_range(bounds.start_col as f32 + 8.0, bounds.end_col as f32 - 8.0);
            let start_y = bounds.end_row as f32 - 1.0 - self.rng.next_range(0.0, 3.0);

            // Palette + accent; occasionally blend a firework color toward the theme accent.
            let mut colors = FIREWORK_COLORS.to_vec();
            colors.push(self.accent);
            let mut color = colors[self.rng.next_usize(colors.len())];
            if self.rng.next_bool(0.35) {
                let t = 0.35 + self.rng.next_f32() * 0.35;
                color = (
                    (color.0 as f32 * (1.0 - t) + self.accent.0 as f32 * t) as u8,
                    (color.1 as f32 * (1.0 - t) + self.accent.1 as f32 * t) as u8,
                    (color.2 as f32 * (1.0 - t) + self.accent.2 as f32 * t) as u8,
                );
            }

            let target_y =
                bounds.start_row as f32 + self.rng.next_range(3.0, bounds.height() as f32 * 0.45);
            let h = start_y - target_y;

            let gravity = 12.0f32;
            let vy = -(2.0 * gravity * h).sqrt();

            let angle = self
                .rng
                .next_range(70.0f32.to_radians(), 110.0f32.to_radians());
            let vx = -vy * angle.cos() / angle.sin() / 0.55;

            self.rockets.push(Rocket {
                x: start_x,
                y: start_y,
                vx,
                vy,
                color,
            });

            // Stagger next launch — never fire the whole sky at once.
            self.launch_cooldown = 0.35 + self.rng.next_f32() * 0.95 + self.host_bias * 0.25;
            self.wave_launches = self.wave_launches.saturating_add(1);

            // After a short wave, take a quiet breath (longer on battery).
            let wave_cap = if self.on_battery {
                2 + self.rng.next_usize(2) as u32
            } else {
                3 + self.rng.next_usize(3) as u32
            };
            if self.wave_launches >= wave_cap {
                self.quiet_timer = 1.8 + self.rng.next_f32() * 2.4 + self.host_bias * 0.6;
                if self.on_battery {
                    self.quiet_timer += 1.0;
                }
                self.wave_launches = 0;
            }
        }
    }

    pub(super) fn update_rockets(&mut self, delta: f32, cols: usize, rows: usize) {
        let mut exploded_rockets = Vec::new();
        for (i, rocket) in self.rockets.iter_mut().enumerate() {
            rocket.x += rocket.vx * delta;
            rocket.y += rocket.vy * delta;

            rocket.vy += 12.0 * delta;
            rocket.vx *= 1.0 - 0.2 * delta;
            rocket.vy *= 1.0 - 0.2 * delta;

            // Ember trail — linger a little longer than before.
            if self.rng.next_bool(0.45) {
                let life = self.rng.next_range(0.28, 0.55);
                self.particles.push(Particle {
                    x: rocket.x,
                    y: rocket.y,
                    vx: self.rng.next_range(-0.3, 0.3),
                    vy: self.rng.next_range(0.1, 0.5),
                    color: (100, 100, 100),
                    ch: '.',
                    life,
                    max_life: life,
                });
            }

            if rocket.vy >= -1.0
                || rocket.y <= 2.0
                || rocket.x <= 1.0
                || rocket.x >= cols as f32 - 1.0
            {
                exploded_rockets.push(i);
            }
        }

        let reach = crate::runner::span_reach_scale(cols, rows);
        let speed_scale = reach.sqrt();
        let bat = if self.on_battery { 0.55 } else { 1.0 };
        for idx in exploded_rockets.into_iter().rev() {
            let rocket = self.rockets.remove(idx);
            let num_particles =
                ((self.rng.next_usize(20) + 20) as f32 * self.quality_scale * bat) as usize;
            for _ in 0..num_particles {
                let angle = self.rng.next_range(0.0, std::f32::consts::TAU);
                let speed = self.rng.next_range(4.0, 16.0) * speed_scale;

                let vx = angle.cos() * speed / 0.55;
                let vy = angle.sin() * speed;

                let ch = match self.rng.next_usize(4) {
                    0 => '*',
                    1 => '+',
                    2 => '•',
                    _ => '.',
                };
                // Soft linger so bursts fade instead of snuffing out.
                let max_life = self.rng.next_range(0.45, 0.95);

                self.particles.push(Particle {
                    x: rocket.x,
                    y: rocket.y,
                    vx,
                    vy,
                    color: rocket.color,
                    ch,
                    life: max_life,
                    max_life,
                });
            }

            // Longer flare life for soft explosion bloom.
            let exp_max_life = self.rng.next_range(0.28, 0.48);
            self.explosions.push(Explosion {
                x: rocket.x,
                y: rocket.y,
                vx: rocket.vx * 0.4,
                vy: rocket.vy * 0.4,
                color: rocket.color,
                life: exp_max_life,
                max_life: exp_max_life,
            });
        }
    }
}
