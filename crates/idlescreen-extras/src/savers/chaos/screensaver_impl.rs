use super::{Chaos, ExplosionType, Particle, Phase, Star};

use crate::runner::{Screensaver, TerminalCell};
use crate::runner::{get_system_info, query_current_palette};
use std::time::Duration;

impl Screensaver for Chaos {
    fn init(&mut self, cols: usize, rows: usize) {
        self.intro_fade = 0.0;
        self.chromatic_strength = 0.0;
        self.phase = Phase::Assembled;
        self.phase_timer = 0.0;
        self.time_elapsed = 0.0;
        self.last_cols = 0;
        self.last_rows = 0;
        // Force rebuild on next update with current size.
        let _ = (cols, rows);
    }

    fn update_frame_time(&mut self, dt: Duration) {
        let dt_secs = dt.as_secs_f32();

        // Exponential moving average for frame time (alpha = 0.1)
        self.frame_time_ema = self.frame_time_ema * 0.9 + dt_secs.min(0.2) * 0.1;

        // Auto-detect the baseline frame time during the startup phase
        if self.time_elapsed < 1.5 {
            self.target_frame_time = self.frame_time_ema;
        }

        if self.time_elapsed > 1.5 {
            let speed_mult = if self.on_battery { 0.65 } else { 1.0 };
            let delta = dt_secs * speed_mult;
            if self.frame_time_ema > self.target_frame_time * 1.25 {
                self.quality_scale = (self.quality_scale - 0.15 * delta).max(0.20);
            } else if self.frame_time_ema < self.target_frame_time * 1.05 {
                self.quality_scale = (self.quality_scale + 0.04 * delta).min(1.0);
            }
        }
    }

    fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        let dt_secs = dt.as_secs_f32().min(0.1);
        let speed_mult = if self.on_battery { 0.65 } else { 1.0 };
        let delta = dt_secs * speed_mult;
        self.phase_timer += delta;
        self.time_elapsed += delta;

        // Intro fade ~0.45s
        if self.intro_fade < 1.0 {
            self.intro_fade = (self.intro_fade + delta / 0.45).min(1.0);
        }

        // Soft chromatic target from phase (ease, never hard cut).
        let chroma_target = match self.phase {
            Phase::Exploding => 0.85,
            Phase::SnapBack => 0.55,
            Phase::Assembled => {
                // Brief soft shimmer during long rest, not a full flash.
                if self.phase_timer > 2.5 && (self.phase_timer % 5.5) < 0.35 {
                    0.28
                } else {
                    0.0
                }
            }
            Phase::Chaos => match self.explosion_type {
                ExplosionType::GlitchWave | ExplosionType::Shockwave => 0.45,
                _ => 0.12,
            },
        };
        let ease = 1.0 - (-delta * 4.5).exp();
        self.chromatic_strength += (chroma_target - self.chromatic_strength) * ease;

        // OpenRGB unstable phase-based updates
        // Live: high system load = more chaos/explosions, host_bias unique instability
        // Support sys_refresh_timer = -1000.0 to prevent slow sys_info calls during tests.
        if self.sys_refresh_timer >= 0.0 {
            self.sys_refresh_timer += delta;
            if self.sys_refresh_timer >= 1.0 {
                let sys = get_system_info();
                self.mem_pressure = sys.mem_used_pct / 100.0;
                self.cpu_load = (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
                self.on_battery = sys.power_status.contains("Battery");
                self.cached_accent = query_current_palette().accent;
                self.sys_refresh_timer = 0.0;
            }
        }

        // Reinitialize if screen size changed
        if cols != self.last_cols || rows != self.last_rows {
            self.refresh_screen_cache(cols, rows);
            self.particles.clear();
            self.stars.clear();
            self.intro_fade = 0.0;
            self.chromatic_strength = 0.0;
            if let Some(logo) =
                crate::runner::place_centered_logo(cols, rows, &get_system_info().logo_text, None)
            {
                for (r_offset, line) in logo.lines.iter().enumerate() {
                    for (c_offset, ch) in line.chars().enumerate() {
                        if ch != ' ' {
                            let mut skip_chance = 0.0;
                            if self.on_battery {
                                skip_chance = 0.50;
                            }
                            let scale_pct = self.quality_scale;
                            if scale_pct < 1.0 {
                                skip_chance = 1.0 - (1.0 - skip_chance) * scale_pct;
                            }
                            if self.particle_limit_opt == 0 {
                                skip_chance = 1.0 - (1.0 - skip_chance) * 0.5;
                            }
                            if skip_chance > 0.0 && self.rng.next_bool(skip_chance) {
                                continue;
                            }
                            let hx = (logo.x + c_offset) as f32;
                            let hy = (logo.y + r_offset) as f32;
                            self.particles.push(Particle {
                                home_x: hx,
                                home_y: hy,
                                x: hx,
                                y: hy,
                                vx: 0.0,
                                vy: 0.0,
                                ch,
                                orig_ch: ch,
                                glow: 0.0,
                                snapped: true,
                            });
                        }
                    }
                }
            }

            self.phase = Phase::Assembled;
            self.phase_timer = 0.0;
            self.last_cols = cols;
            self.last_rows = rows;
        }

        // Dynamically adjust star population to match target capacity
        let bat = if self.on_battery { 0.55 } else { 1.0 };
        let target_stars =
            (((cols * rows / 16).clamp(20, 100)) as f32 * self.quality_scale * bat) as usize;
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
                });
            }
        }

        // Decay star excitations
        for star in &mut self.stars {
            if star.excitation > 0.0 {
                star.excitation -= delta * 2.5;
                if star.excitation < 0.0 {
                    star.excitation = 0.0;
                }
            }
        }

        // Particle-star proximity interaction: unsnapped particles excite nearby stars
        let cols_f = cols as f32;
        let rows_f = rows as f32;
        let star_excite_mult = match self.explosion_type {
            ExplosionType::Shockwave => 2.2,
            ExplosionType::Entropy => 1.8,
            ExplosionType::Resonance => 1.4,
            ExplosionType::BlackHole => 0.7,
            _ => 1.5,
        };
        for p in &self.particles {
            if !p.snapped {
                for star in &mut self.stars {
                    let sx = star.x * cols_f;
                    let sy = star.y * rows_f;
                    let dx = p.x - sx;
                    let dy = (p.y - sy) * 2.0;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < 9.0 {
                        let dist = dist_sq.sqrt();
                        let force = (1.0 - dist / 3.0) * 1.5 * star_excite_mult;
                        star.excitation = star.excitation.max(force);
                    }
                }
            }
        }

        // Particle dynamics update based on phase
        match self.phase {
            Phase::Assembled => {
                self.update_assembled(delta);
            }
            Phase::Exploding => {
                self.update_exploding(cols, rows);
            }
            Phase::Chaos => {
                self.update_chaos(delta, cols, rows);
            }
            Phase::SnapBack => {
                self.update_snapback(delta);
            }
        }
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        self.draw_impl(grid, cols, rows);
    }
}
