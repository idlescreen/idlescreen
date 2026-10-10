//! Aurora borealis — slow curtains of light over a twinkling starfield.

mod draw;
mod sky;
mod types;

pub use types::{Curtain, Star};

use crate::runner::Screensaver;
use crate::runner::{LcgRng, TerminalCell};
use crate::runner::{get_system_info, query_current_palette};
use std::cell::RefCell;
use std::time::Duration;

pub struct Aurora {
    pub(crate) rng: LcgRng,
    pub(crate) curtains: Vec<Curtain>,
    pub(crate) stars: Vec<Star>,
    pub(crate) time_elapsed: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,
    pub(super) on_battery: bool,
    pub(super) frame_time_ema: f32,
    pub(super) quality_scale: f32,
    pub(super) target_frame_time: f32,
    pub(crate) sys_refresh_timer: f32,
    pub(crate) mem_pressure: f32,
    pub(crate) cpu_load: f32,
    pub(crate) logo_text: String,
    pub(crate) accent: (u8, u8, u8),
    pub(crate) target_accent: (u8, u8, u8),
    pub(crate) current_accent: (f32, f32, f32),
    /// 0→1 fade-in after init / resize (~0.45s)
    pub(crate) intro_fade: f32,
    /// Countdown to the next brightening event.
    pub(crate) surge_timer: f32,
    /// How long the current surge stays bright.
    pub(crate) surge_hold: f32,
    /// Eased 0→1 surge brightness envelope.
    pub(crate) surge_env: f32,
    /// User overrides: `aurora.curtains` (1..=3), `aurora.surge` (0|1).
    pub(crate) curtains_opt: u32,
    pub(crate) surge_opt: bool,
    /// Per-column edge heights and ray modulation (cols × n_curtains).
    pub(crate) edge_scratch: RefCell<Vec<f32>>,
    pub(crate) ray_scratch: RefCell<Vec<f32>>,
    /// Per-cell intensity + weighted hue accumulators (cols × rows).
    pub(crate) field_scratch: RefCell<Vec<f32>>,
    pub(crate) hue_scratch: RefCell<Vec<f32>>,
}

impl Default for Aurora {
    fn default() -> Self {
        Self::new()
    }
}

impl Aurora {
    pub fn new() -> Self {
        let rng = LcgRng::from_env_or_random();
        let sys = get_system_info();
        let on_battery = sys.power_status.contains("Battery");
        let accent = query_current_palette().accent;
        let host_bias = sys.hostname.chars().map(|c| c as u32).sum::<u32>() as f32 / 1000.0 % 1.0;
        let mut rng2 = rng;
        let curtains = sky::seed_curtains(&mut rng2, host_bias);
        let curtains_opt = crate::runner::param_f32("aurora.curtains")
            .or_else(|| crate::runner::param_f32("curtains"))
            .map_or(3, |v| v.clamp(1.0, 3.0) as u32);
        let surge_opt = crate::runner::param_f32("aurora.surge")
            .or_else(|| crate::runner::param_f32("surge"))
            != Some(0.0);
        Self {
            rng: rng2,
            curtains,
            stars: Vec::new(),
            time_elapsed: 0.0,
            last_cols: 0,
            last_rows: 0,
            on_battery,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,
            sys_refresh_timer: 0.0,
            mem_pressure: sys.mem_used_pct / 100.0,
            cpu_load: (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0),
            logo_text: sys.logo_text,
            accent,
            target_accent: accent,
            current_accent: (accent.0 as f32, accent.1 as f32, accent.2 as f32),
            intro_fade: 0.0,
            surge_timer: 14.0,
            surge_hold: 0.0,
            surge_env: 0.0,
            curtains_opt,
            surge_opt,
            edge_scratch: RefCell::new(Vec::new()),
            ray_scratch: RefCell::new(Vec::new()),
            field_scratch: RefCell::new(Vec::new()),
            hue_scratch: RefCell::new(Vec::new()),
        }
    }

    /// Pin the saver into a stable state for bench harness measurements.
    /// See `cosmos::Cosmos::prepare_for_bench` for the rationale; the
    /// important bit is `sys_refresh_timer = -1000.0` to suppress the
    /// slow system-info probe inside `update()`.
    pub fn prepare_for_bench(&mut self, cols: usize, rows: usize) {
        self.sys_refresh_timer = -1000.0;
        self.last_cols = cols;
        self.last_rows = rows;
    }

    fn update_frame_time_impl(&mut self, dt: Duration) {
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

impl Screensaver for Aurora {
    fn init(&mut self, cols: usize, rows: usize) {
        self.intro_fade = 0.0;
        self.last_cols = cols;
        self.last_rows = rows;
        self.time_elapsed = 0.0;
        self.surge_timer = 14.0 + self.rng.next_f32() * 10.0;
        self.surge_hold = 0.0;
        self.surge_env = 0.0;
        self.stars = sky::seed_stars(&mut self.rng, cols, rows, self.quality_scale);
    }

    fn update_frame_time(&mut self, dt: Duration) {
        self.update_frame_time_impl(dt);
    }

    fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        let dt_secs = dt.as_secs_f32().min(0.1);
        let speed_mult = if self.on_battery { 0.65 } else { 1.0 };
        // CPU load nudges the flow rate; heavy load never freezes it.
        let flow = 0.75 + 0.55 * self.cpu_load;
        let delta = dt_secs * speed_mult * flow;
        self.time_elapsed += delta;

        if self.intro_fade < 1.0 {
            self.intro_fade = (self.intro_fade + dt_secs * speed_mult / 0.45).min(1.0);
        }

        // Surge envelope: rare ~5–9s brightenings, eased in and out.
        if self.surge_opt {
            self.surge_timer -= dt_secs * speed_mult;
            self.surge_hold -= dt_secs * speed_mult;
            if self.surge_timer <= 0.0 {
                self.surge_hold = self.rng.next_range(5.0, 9.0);
                self.surge_timer = self.rng.next_range(18.0, 40.0);
            }
            let target = if self.surge_hold > 0.0 { 1.0 } else { 0.0 };
            let ease = 1.0 - (-dt_secs * speed_mult * 1.4).exp();
            self.surge_env += (target - self.surge_env) * ease;
            self.curtains[3].strength = self.surge_env * 1.15;
        } else {
            self.surge_env = 0.0;
            self.curtains[3].strength = 0.0;
        }

        self.sys_refresh_timer += dt_secs * speed_mult;
        if self.sys_refresh_timer >= 1.0 {
            let sys = get_system_info();
            self.mem_pressure = sys.mem_used_pct / 100.0;
            self.cpu_load = (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
            self.on_battery = sys.power_status.contains("Battery");
            self.logo_text = sys.logo_text;
            self.target_accent = query_current_palette().accent;
            self.sys_refresh_timer = 0.0;
        }

        // Smooth 1-second RGB lerp toward target accent
        let t = (dt_secs * speed_mult / 1.0).clamp(0.0, 1.0);
        self.current_accent.0 += (self.target_accent.0 as f32 - self.current_accent.0) * t;
        self.current_accent.1 += (self.target_accent.1 as f32 - self.current_accent.1) * t;
        self.current_accent.2 += (self.target_accent.2 as f32 - self.current_accent.2) * t;
        self.accent = (
            self.current_accent.0.round() as u8,
            self.current_accent.1.round() as u8,
            self.current_accent.2.round() as u8,
        );

        if cols != self.last_cols || rows != self.last_rows {
            self.last_cols = cols;
            self.last_rows = rows;
            self.intro_fade = 0.0;
            self.stars = sky::seed_stars(&mut self.rng, cols, rows, self.quality_scale);
        }

        // Memory pressure trims the mid curtain; battery trims the far one.
        let active_base = if self.mem_pressure > 0.85 {
            2
        } else {
            self.curtains_opt.max(1) as usize
        };
        for (i, c) in self.curtains.iter_mut().take(3).enumerate() {
            let base = [1.0f32, 0.85, 0.7][i];
            c.strength = if i < active_base {
                base * if self.on_battery && i == 2 { 0.45 } else { 1.0 }
            } else {
                0.0
            };
        }
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        draw::draw_aurora(self, grid, cols, rows);
    }
}

#[cfg(test)]
#[path = "aurora_tests.rs"]
mod tests;
