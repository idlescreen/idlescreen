//! Cozy fireplace — multi-tongue flame, coal bed, smoke.
//! OS name only appears where smoke drifts across it.

mod background;
mod draw;
mod fx;
mod particles;
mod setup;
mod types;

pub use types::{Ember, Log, Smoke, Tongue};

use crate::runner::Screensaver;
use crate::runner::{LcgRng, TerminalCell};
use crate::runner::{get_system_info, query_current_palette};
use std::time::Duration;

pub struct Hearth {
    rng: LcgRng,
    time: f32,
    intro_fade: f32,
    last_cols: usize,
    last_rows: usize,
    embers: Vec<Ember>,
    smoke: Vec<Smoke>,
    logs: Vec<Log>,
    tongues: Vec<Tongue>,
    spawn_timer: f32,
    smoke_timer: f32,
    coal_boost: f32,
    on_battery: bool,
    quality_scale: f32,
    frame_time_ema: f32,
    target_frame_time: f32,
    sys_timer: f32,
    accent: (u8, u8, u8),
    logo_text: String,
    kernel: String,
    fire_cx: f32,
    fire_y: f32,
    fire_w: f32,
    mantel_y: usize,
}

impl Default for Hearth {
    fn default() -> Self {
        Self::new()
    }
}

impl Hearth {
    pub fn new() -> Self {
        let sys = get_system_info();
        Self {
            rng: LcgRng::from_env_or_random(),
            time: 0.0,
            intro_fade: 0.0,
            last_cols: 0,
            last_rows: 0,
            embers: Vec::new(),
            smoke: Vec::new(),
            logs: Vec::new(),
            tongues: Vec::new(),
            spawn_timer: 0.0,
            smoke_timer: 0.0,
            coal_boost: 0.0,
            on_battery: sys.power_status.contains("Battery"),
            quality_scale: 1.0,
            frame_time_ema: 0.016,
            target_frame_time: 0.016,
            sys_timer: 0.0,
            accent: query_current_palette().accent,
            logo_text: sys.logo_text,
            kernel: sys.kernel,
            fire_cx: 40.0,
            fire_y: 18.0,
            fire_w: 24.0,
            mantel_y: 6,
        }
    }

    /// Pin the saver into a stable state for bench harness measurements.
    /// See `cosmos::Cosmos::prepare_for_bench` for the rationale; the
    /// important bit is `sys_refresh_timer = -1000.0` to suppress the
    /// slow system-info probe inside `update()`.
    pub fn prepare_for_bench(&mut self, cols: usize, rows: usize) {
        self.sys_timer = -1000.0;
        self.last_cols = cols;
        self.last_rows = rows;
    }
}

impl Screensaver for Hearth {
    fn init(&mut self, cols: usize, rows: usize) {
        self.intro_fade = 0.0;
        self.time = 0.0;
        self.last_cols = cols;
        self.last_rows = rows;
        self.embers.clear();
        self.smoke.clear();
        self.logs.clear();
        self.tongues.clear();
        self.spawn_timer = 0.0;
        self.smoke_timer = 0.0;

        let layout = setup::create_layout(&mut self.rng, cols, rows);
        self.fire_cx = layout.fire_cx;
        self.fire_y = layout.fire_y;
        self.fire_w = layout.fire_w;
        self.mantel_y = layout.mantel_y;
        self.logs = layout.logs;
        self.tongues = layout.tongues;

        for _ in 0..25 {
            particles::spawn_ember(
                &mut self.embers,
                &mut self.rng,
                self.fire_cx,
                self.fire_y,
                self.fire_w,
                cols,
            );
        }
        for _ in 0..12 {
            particles::spawn_smoke(
                &mut self.smoke,
                &mut self.rng,
                self.fire_cx,
                self.fire_y,
                self.fire_w,
                cols,
            );
        }
    }

    fn update_frame_time(&mut self, dt: Duration) {
        let dt_secs = dt.as_secs_f32();
        self.frame_time_ema = self.frame_time_ema * 0.9 + dt_secs.min(0.2) * 0.1;
        if self.time > 1.5 {
            let d = dt_secs * if self.on_battery { 0.65 } else { 1.0 };
            if self.frame_time_ema > self.target_frame_time * 1.15 {
                self.quality_scale = (self.quality_scale - 0.12 * d).max(0.3);
            } else if self.frame_time_ema < self.target_frame_time * 1.05 {
                self.quality_scale = (self.quality_scale + 0.04 * d).min(1.0);
            }
        }
    }

    fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        let dt_secs = dt.as_secs_f32().min(0.1);
        let speed = if self.on_battery { 0.8 } else { 1.0 };
        let delta = dt_secs * speed;
        self.time += delta;
        if self.intro_fade < 1.0 {
            self.intro_fade = (self.intro_fade + delta / 0.45).min(1.0);
        }
        if cols != self.last_cols || rows != self.last_rows {
            self.init(cols, rows);
            return;
        }

        self.sys_timer += delta;
        if self.sys_timer >= 1.5 {
            self.sys_timer = 0.0;
            let sys = get_system_info();
            self.on_battery = sys.power_status.contains("Battery");
            self.accent = query_current_palette().accent;
            self.logo_text = sys.logo_text;
            self.kernel = sys.kernel;
        }

        self.coal_boost = (self.coal_boost - delta * 1.2).max(0.0);

        for t in &mut self.tongues {
            t.phase += t.speed * delta;
        }
        for log in &mut self.logs {
            log.phase += delta * 2.0;
        }

        particles::update_particles(
            &mut self.embers,
            &mut self.smoke,
            &mut self.rng,
            cols,
            delta,
        );

        let max_embers =
            ((if self.on_battery { 40.0 } else { 90.0 }) * self.quality_scale) as usize;
        let max_smoke = ((if self.on_battery { 18.0 } else { 40.0 }) * self.quality_scale) as usize;

        particles::update_spawners(
            &mut self.embers,
            &mut self.smoke,
            &mut self.rng,
            &mut self.spawn_timer,
            &mut self.smoke_timer,
            self.fire_cx,
            self.fire_y,
            self.fire_w,
            cols,
            max_embers,
            max_smoke,
            self.on_battery,
            delta,
        );

        if self.rng.next_f32() < delta * 0.4 && self.embers.len() + 5 < max_embers {
            self.coal_boost = 1.0;
            particles::spawn_burst(
                &mut self.embers,
                &mut self.rng,
                self.fire_cx,
                self.fire_y,
                self.fire_w,
                cols,
            );
        }
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        draw::draw_impl(
            grid,
            cols,
            rows,
            &self.embers,
            &self.smoke,
            &self.logs,
            &self.tongues,
            self.fire_cx,
            self.fire_y,
            self.fire_w,
            self.mantel_y,
            self.intro_fade,
            self.time,
            self.coal_boost,
            self.accent,
            &self.logo_text,
            &self.kernel,
        );
    }
}

#[cfg(test)]
#[path = "hearth_tests.rs"]
mod tests;
