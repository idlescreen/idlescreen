// SPDX-License-Identifier: MIT
// `Screensaver` trait impl for `Storm` — init, update_frame_time,
// update, draw. Each method is a self-contained entry point; they
// live in one page because the trait requires them co-located and
// they share the `&mut self` / `&self` receiver shape.

use std::time::Duration;

use crate::runner::{Screensaver, TerminalCell, get_system_info, query_current_palette};

use super::Storm;

impl Screensaver for Storm {
    fn update_frame_time(&mut self, dt: Duration) {
        let dt_secs = dt.as_secs_f32();

        // Auto-detect high refresh rates during the startup phase
        if self.phase_timer < 2.0 && dt_secs > 0.001 && dt_secs < self.target_frame_time - 0.001 {
            self.target_frame_time = dt_secs;
        }

        // Exponential moving average for frame time (alpha = 0.1)
        self.frame_time_ema = self.frame_time_ema * 0.9 + dt_secs.min(0.2) * 0.1;

        if self.phase_timer > 1.5 {
            let speed_mult = if self.on_battery { 0.65 } else { 1.0 };
            let delta = dt_secs * speed_mult;
            if self.frame_time_ema > self.target_frame_time * 1.15 {
                self.quality_scale = (self.quality_scale - 0.15 * delta).max(0.20);
            } else if self.frame_time_ema < self.target_frame_time * 1.05 {
                self.quality_scale = (self.quality_scale + 0.04 * delta).min(1.0);
            }
        }
    }

    fn init(&mut self, _cols: usize, _rows: usize) {
        // Leave last_cols/last_rows so the next update's check_resize rebuilds scenery.
        self.intro_fade = 0.0;
        self.time_elapsed = 0.0;
        self.phase_timer = 0.0;
        self.drops.clear();
        self.splashes.clear();
        self.lightning_flash = 0.0;
        self.lightning_bolts.clear();
        self.last_cols = 0;
        self.last_rows = 0;
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

        // Wind target drifts with slow LFO; actual wind eases toward it (no snaps)
        self.wind_target =
            (self.phase_timer * 0.35).sin() * 9.0 + (self.phase_timer * 1.5).cos() * 2.0;
        let wind_ease = 1.0 - (-delta * 1.4).exp();
        self.wind += (self.wind_target - self.wind) * wind_ease;

        self.sys_refresh_timer += delta;
        if self.sys_refresh_timer >= 1.0 {
            let sys = get_system_info();
            self.mem_pressure = sys.mem_used_pct / 100.0;
            self.cpu_load = (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
            self.on_battery = sys.power_status.contains("Battery");
            self.cached_accent = query_current_palette().accent;
            self.sys_refresh_timer = 0.0;
        }

        self.check_resize(cols, rows);

        let load_mult = 1.0 + self.cpu_load * 0.6 + self.mem_pressure * 0.3;
        let speed_mult = match self.assemble_speed_opt {
            0 => 0.6f32,
            2 => 1.6f32,
            _ => 1.0f32,
        } * load_mult;

        self.update_drops(delta, cols, rows, speed_mult);
        if !crate::runner::is_secondary_monitor() {
            self.update_bird(delta, cols, rows);
            self.update_scenery_and_animals(delta, cols, rows);
        }
        self.update_lightning(delta, cols, rows);
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        self.draw_impl(grid, cols, rows);
    }
}
