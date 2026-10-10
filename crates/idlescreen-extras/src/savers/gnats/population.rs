use super::{Firefly, Gnats, Star};
use crate::runner::{hsl_to_rgb, rgb_to_hsl};

impl Gnats {
    pub(crate) fn spawn_new_firefly(&mut self, cols: usize, rows: usize) {
        let size = self.rng.next_range(0.0, 4.0) as u8;
        let speed_mult = self.rng.next_range(0.7, 1.3);

        // library 4.0: pull from the canonical ScreenPalette.
        let accent = self.accent;
        let (acc_h, _acc_s, _acc_l) = rgb_to_hsl(accent.0, accent.1, accent.2);
        let p = self.rng.next_f32();
        let h = if p < 0.4 {
            (acc_h + self.rng.next_range(-15.0, 15.0)).rem_euclid(360.0)
        } else if p < 0.7 {
            (acc_h + 120.0 + self.rng.next_range(-15.0, 15.0)).rem_euclid(360.0)
        } else {
            (acc_h - 120.0 + self.rng.next_range(-15.0, 15.0)).rem_euclid(360.0)
        };
        let color = hsl_to_rgb(h, 0.95, 0.60);

        // Spawn on the bottom of the screen to make it feel like they wake up and take flight
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
        let x = self
            .rng
            .next_range(bounds.start_col as f32, bounds.end_col as f32);
        let y = bounds.end_row as f32 - 1.0;

        self.fireflies.push(Firefly {
            x,
            y,
            vx: self.rng.next_range(-3.0, 3.0),
            vy: self.rng.next_range(-5.0, -1.0), // Initial upward velocity
            color,
            size,
            speed_mult,
            history: Vec::new(),
            blink_phase: self.rng.next_f32() * std::f32::consts::TAU,
            blink_rate: self.rng.next_range(1.6, 3.8),
        });
    }

    pub(crate) fn adjust_populations(&mut self, cols: usize, rows: usize) {
        // Dynamically adjust fireflies to match target capacity
        let num_fireflies = (((cols * rows) / 45).clamp(30, 60) as f32
            * self.quality_scale
            * (if self.on_battery { 0.55 } else { 1.0 })) as usize;
        if self.fireflies.len() > num_fireflies {
            self.fireflies.truncate(num_fireflies);
        } else if self.fireflies.len() < num_fireflies && num_fireflies > 0 {
            while self.fireflies.len() < num_fireflies {
                self.spawn_new_firefly(cols, rows);
            }
        }

        // Dynamically adjust star population to match target capacity
        let target_stars = (((cols * rows) / 25).clamp(30, 120) as f32
            * self.quality_scale
            * (if self.on_battery { 0.55 } else { 1.0 })) as usize;
        if self.stars.len() > target_stars {
            self.stars.truncate(target_stars);
        } else if self.stars.len() < target_stars && target_stars > 0 {
            while self.stars.len() < target_stars {
                let ch = if self.stars.len().is_multiple_of(8) {
                    '✦'
                } else if self.stars.len().is_multiple_of(3) {
                    '+'
                } else {
                    '.'
                };
                self.stars.push(Star {
                    x: self.rng.next_f32(),
                    y: self.rng.next_f32(),
                    phase: self.rng.next_f32() * std::f32::consts::TAU,
                    ch,
                    excitation: 0.0,
                });
            }
        }
    }

    pub(crate) fn update_frame_time_impl(&mut self, dt: std::time::Duration) {
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
}
