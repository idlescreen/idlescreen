//! Consolidated chaos screensaver effect module — hero visual polish.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).

use crate::runner::LcgRng;
use crate::runner::{get_system_info, query_current_palette};

mod draw;
mod physics;
mod screensaver_impl;
mod types;
mod update_chaos;

#[cfg(test)]
#[path = "chaos_tests.rs"]
mod tests;

pub use types::{ExplosionType, Particle, Phase, Star};

pub struct Chaos {
    pub(crate) rng: LcgRng,
    pub(crate) particles: Vec<Particle>,
    pub(crate) stars: Vec<Star>,
    pub(crate) phase: Phase,
    pub(crate) phase_timer: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,
    pub(crate) explosion_type: ExplosionType,
    pub(crate) black_hole_burst_triggered: bool,
    pub(crate) particle_limit_opt: u32,
    pub(crate) explosion_freq_opt: u32,

    // Live system dynamics
    pub(crate) sys_refresh_timer: f32,
    pub(crate) mem_pressure: f32,
    pub(crate) cpu_load: f32,
    pub(crate) host_bias: f32,
    pub(super) on_battery: bool,
    pub(super) frame_time_ema: f32,
    pub(super) quality_scale: f32,
    pub(super) target_frame_time: f32,

    pub(crate) time_elapsed: f32,
    pub(crate) cached_accent: (u8, u8, u8),
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
    pub(crate) max_possible_dist: f32,
    pub(crate) inv_max_possible_dist: f32,

    /// 0→1 fade-in after init / resize (~0.45s)
    pub(crate) intro_fade: f32,
    /// Soft chromatic aberration strength (eases in/out; never hard-cuts).
    pub(crate) chromatic_strength: f32,
}

impl Default for Chaos {
    fn default() -> Self {
        Self::new()
    }
}

impl Chaos {
    pub fn new() -> Self {
        let particle_limit_opt: u32 = 1;
        let explosion_freq_opt: u32 = 1;

        let sys = get_system_info();
        let host_bias = sys.hostname.chars().map(|c| c as u32).sum::<u32>() as f32 / 1000.0 % 1.0;
        let on_battery = sys.power_status.contains("Battery");

        Self {
            rng: LcgRng::from_env_or_random(),
            particles: Vec::new(),
            stars: Vec::new(),
            phase: Phase::Assembled,
            phase_timer: 0.0,
            last_cols: 0,
            last_rows: 0,
            explosion_type: ExplosionType::Supernova,
            black_hole_burst_triggered: false,
            particle_limit_opt,
            explosion_freq_opt,
            sys_refresh_timer: 0.0,
            mem_pressure: sys.mem_used_pct / 100.0,
            cpu_load: (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0),
            host_bias,
            on_battery,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,

            time_elapsed: 0.0,
            cached_accent: query_current_palette().accent,
            center_x: 0.0,
            center_y: 0.0,
            max_possible_dist: 1.0,
            inv_max_possible_dist: 1.0,
            intro_fade: 0.0,
            chromatic_strength: 0.0,
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

    pub(crate) fn refresh_screen_cache(&mut self, cols: usize, rows: usize) {
        let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
        self.center_x = (primary.start_col + primary.width() / 2) as f32;
        self.center_y = (primary.start_row + primary.height() / 2) as f32;
        self.max_possible_dist = (self.center_x * self.center_x + self.center_y * self.center_y)
            .sqrt()
            .max(1.0);
        self.inv_max_possible_dist = 1.0 / self.max_possible_dist;
    }
}
