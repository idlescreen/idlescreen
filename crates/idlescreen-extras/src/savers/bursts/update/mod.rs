//! Screensaver update implementation for Bursts.

mod particles;
mod rockets;

use crate::bursts::Bursts;
use crate::runner::Screensaver;
use std::time::Duration;

impl Screensaver for Bursts {
    fn init(&mut self, cols: usize, rows: usize) {
        self.intro_fade = 0.0;
        self.last_cols = cols;
        self.last_rows = rows;
        self.rockets.clear();
        self.particles.clear();
        self.explosions.clear();
        self.stars.clear();
        self.time_elapsed = 0.0;
        self.launch_cooldown = 0.4 + self.host_bias * 0.5;
        self.quiet_timer = 0.0;
        self.wave_launches = 0;
        self.generate_skyline(cols, rows);
    }

    fn update_frame_time(&mut self, dt: Duration) {
        let dt_secs = dt.as_secs_f32();

        if self.time_elapsed < 2.0 && dt_secs > 0.001 && dt_secs < self.target_frame_time - 0.001 {
            self.target_frame_time = dt_secs;
        }

        self.frame_time_ema = self.frame_time_ema * 0.9 + dt_secs.min(0.2) * 0.1;

        if self.time_elapsed > 1.5 {
            let speed_mult = if self.on_battery { 0.65 } else { 1.0 };
            let delta = dt_secs * speed_mult;
            if self.frame_time_ema > self.target_frame_time * 1.15 {
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
        self.time_elapsed += delta;

        // Intro fade ~0.45s
        if self.intro_fade < 1.0 {
            self.intro_fade = (self.intro_fade + delta / 0.45).min(1.0);
        }

        if self.launch_cooldown > 0.0 {
            self.launch_cooldown = (self.launch_cooldown - delta).max(0.0);
        }
        if self.quiet_timer > 0.0 {
            self.quiet_timer = (self.quiet_timer - delta).max(0.0);
        }

        self.refresh_live_stats(delta);
        self.resize_if_needed(cols, rows);
        self.sync_star_population(cols, rows);
        self.maybe_launch_rocket(cols, rows);
        self.update_rockets(delta, cols, rows);
        self.decay_star_excitations(delta);
        self.update_particles_and_stars(delta, cols, rows);
        self.update_explosion_entities(delta);
        self.twinkle_skyline_windows(delta);
    }

    fn draw(&self, grid: &mut [crate::runner::TerminalCell], cols: usize, rows: usize) {
        self.draw_impl(grid, cols, rows);
    }
}
