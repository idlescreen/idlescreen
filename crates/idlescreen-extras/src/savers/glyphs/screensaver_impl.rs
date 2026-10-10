use super::Glyphs;
use super::physics;
use crate::runner::Screensaver;
use crate::runner::TerminalCell;
use crate::runner::{get_system_info, query_current_palette};
use std::time::Duration;

impl Screensaver for Glyphs {
    fn init(&mut self, cols: usize, rows: usize) {
        self.intro_fade = 0.0;
        self.last_cols = cols;
        self.last_rows = rows;
        self.drops.clear();
        self.time_elapsed = 0.0;
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

        self.sys_refresh_timer += delta;
        if self.sys_refresh_timer >= 2.0 {
            self.sys_refresh_timer = 0.0;
            let sys = get_system_info();
            self.mem_pressure = sys.mem_used_pct / 100.0;
            self.on_battery = sys.power_status.contains("Battery");
            self.logo_text = sys.logo_text;
            self.cached_accent = query_current_palette().accent;
        }

        if cols != self.last_cols || rows != self.last_rows {
            self.last_cols = cols;
            self.last_rows = rows;
            self.drops.clear();
            self.intro_fade = 0.0;
        }

        // Density breathes gently so the field never locks into a fixed grid.
        let density_breathe = 0.88 + 0.12 * (self.time_elapsed * 0.22).sin();
        let density = self.density_opt.max(1) as usize;
        let target_drops = ((((cols / density.max(1)) as f32)
            * self.quality_scale
            * (if self.on_battery { 0.55 } else { 1.0 })
            * density_breathe) as usize)
            .max(1);

        if self.drops.len() > target_drops {
            self.drops.truncate(target_drops);
        } else if self.drops.len() < target_drops && target_drops > 0 {
            let mut used_cols = vec![false; cols];
            for d in &self.drops {
                if (d.x as usize) < cols {
                    used_cols[d.x as usize] = true;
                }
            }
            let mut unused: Vec<usize> = (0..cols).filter(|x| !used_cols[*x]).collect();
            while self.drops.len() < target_drops && !unused.is_empty() {
                let idx = self.rng.next_usize(unused.len());
                let x = unused.swap_remove(idx);
                self.drops.push(physics::spawn_drop(
                    x,
                    rows,
                    self.char_pool.len(),
                    &mut self.rng,
                ));
            }
        }

        for d in &mut self.drops {
            physics::update_drop(
                d,
                delta,
                rows,
                self.char_pool.len(),
                self.time_elapsed,
                &mut self.rng,
            );
        }
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        self.draw_impl(grid, cols, rows);
    }
}
