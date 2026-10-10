//! Cosmos screensaver: emergent universe lifecycle with live OS caption.
//! Darkness → Big Bang (caption dissolves) → Expansion → Accretion →
//! Singularity → Collapse → Darkness. Hero visual polish.

pub mod physics;
pub mod render;
pub mod types;

pub use types::{GravityCenter, LogoPixel, Particle, UniverseState};

use crate::runner::Screensaver;
use crate::runner::get_system_info;
use crate::runner::{LcgRng, TerminalCell};
use std::time::Duration;

pub struct Cosmos {
    pub(crate) rng: LcgRng,
    pub(crate) state: UniverseState,
    pub(crate) state_timer: f32,
    pub(crate) particles: Vec<Particle>,
    pub(crate) seeds: Vec<GravityCenter>,
    pub(crate) logo_pixels: Vec<LogoPixel>,
    pub(crate) time_elapsed: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,

    // Settings
    pub(crate) seed_density_opt: u32,
    pub(crate) sim_speed_opt: u32,

    // Live system dynamics
    pub(crate) sys_refresh_timer: f32,
    pub(crate) mem_pressure: f32,
    pub(crate) cpu_load: f32,
    pub(crate) on_battery: bool,
    pub(crate) host_bias: f32,
    pub(crate) universe_cx: f32,
    pub(crate) universe_cy: f32,
    pub(crate) spin_clockwise: bool,
    pub(crate) zoom: f32,
    /// Scatter radius for logo pixels during Big Bang (1.0 = one cell per character).
    pub(crate) logo_scale: f32,
    pub(crate) grav_wave_timer: f32,
    pub(crate) grav_wave_cx: f32,
    pub(crate) grav_wave_cy: f32,
    /// Full grav-wave lifetime for soft intensity fade (seconds).
    pub(crate) grav_wave_max: f32,
    pub(crate) frame_time_ema: f32,
    pub(crate) quality_scale: f32,
    pub(crate) target_frame_time: f32,
    /// Cached primary monitor center in grid cells (avoids per-pixel bounds lookup).
    pub(crate) screen_cx: f32,
    pub(crate) screen_cy: f32,
    pub(crate) cached_accent: (u8, u8, u8),
    pub(crate) inv_mass_scratch: Vec<f32>,
    pub(crate) particle_screen_scratch: Vec<(i32, i32)>,

    /// 0→1 fade-in after init / resize (~0.45s)
    pub(crate) intro_fade: f32,
    /// 0→1 ease after each state enter (softens hard phase cuts).
    pub(crate) state_fade: f32,
}

impl Default for Cosmos {
    fn default() -> Self {
        Self::new()
    }
}

impl Cosmos {
    pub fn new() -> Self {
        let seed_density_opt: u32 = 3;
        let sim_speed_opt: u32 = 1;

        let sys = get_system_info();
        let host_bias = sys.hostname.chars().map(|c| c as u32).sum::<u32>() as f32 / 1000.0 % 1.0;
        let on_battery = sys.power_status.contains("Battery");

        Self {
            rng: LcgRng::from_env_or_random(),
            state: UniverseState::Darkness,
            state_timer: 0.0,
            // Particle budget cap is 580 × quality_scale × battery-factor
            // (see physics/particle_cap.rs). Pre-allocate to the cap so
            // ramp-up to budget doesn't realloc 4–5 times.
            particles: Vec::with_capacity(580),
            // Gravity-center seeds rarely exceed 8 per state. Pre-allocate
            // to that ceiling so state-flips don't realloc.
            seeds: Vec::with_capacity(8),
            // Logo pixels depend on screen size and the bitmap raster;
            // 4096 is a comfortably-over ceiling (200×200 bitmap at
            // worst — actual usage tops ~2000 on a 1080p terminal).
            logo_pixels: Vec::with_capacity(4096),
            time_elapsed: 0.0,
            last_cols: 0,
            last_rows: 0,
            seed_density_opt,
            sim_speed_opt,
            sys_refresh_timer: 0.0,
            mem_pressure: sys.mem_used_pct / 100.0,
            cpu_load: 0.4,
            on_battery,
            host_bias,
            universe_cx: 0.0,
            universe_cy: 0.0,
            spin_clockwise: true,
            zoom: 0.58,
            logo_scale: 0.2,
            grav_wave_timer: 0.0,
            grav_wave_cx: 0.0,
            grav_wave_cy: 0.0,
            grav_wave_max: 1.6,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,
            screen_cx: 0.0,
            screen_cy: 0.0,
            cached_accent: (0, 191, 255),
            // Particle-budget cap is 580 × quality_scale × battery-factor
            // (see physics/particle_cap.rs). Pre-allocate the scratch buffers
            // so `clear(); reserve(particles.len());` at the hot path doesn't
            // grow past the typical size once per saver session.
            inv_mass_scratch: Vec::with_capacity(580),
            particle_screen_scratch: Vec::with_capacity(580),
            intro_fade: 0.0,
            state_fade: 1.0,
        }
    }

    /// Pin the saver into the `Accretion` state with a fully-populated
    /// particle field, for deterministic bench harness measurements.
    ///
    /// The public `Screensaver` trait only exposes `init/update/draw`;
    /// warm-up cycles via `update()` reach Accretion eventually but with
    /// jitter from the state-machine timer. `bench_exports::Cosmos`
    /// (the `pub use` re-export in `lib.rs`) lets the bench target poke
    /// private fields directly, but the convention here is one named
    /// method instead of N `pub` field annotations.
    ///
    /// `sys_refresh_timer = -1000.0` matches the skip used by
    /// `tests_perf::test_screensaver_perf_benchmark` — it suppresses
    /// the slow system-info probe inside `update()`.
    pub fn prepare_for_bench(&mut self, cols: usize, rows: usize) {
        self.sys_refresh_timer = -1000.0;
        self.refresh_screen_cache(cols, rows);
        self.state = UniverseState::Accretion;
        self.state_timer = 2.0;
    }

    pub(crate) fn refresh_screen_cache(&mut self, cols: usize, rows: usize) {
        let (cx, cy) = if crate::runner::is_secondary_monitor() {
            (cols as f32 / 2.0, rows as f32 / 2.0)
        } else {
            let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
            (
                (primary.start_col + primary.width() / 2) as f32,
                (primary.start_row + primary.height() / 2) as f32,
            )
        };
        self.screen_cx = cx;
        self.screen_cy = cy;

        let cells = cols.saturating_mul(rows);
        let grid_cap = if cells > 10_000 {
            0.65
        } else if cells > 6_000 {
            0.78
        } else {
            1.0
        };
        self.quality_scale = self.quality_scale.min(grid_cap);
    }
}

impl Screensaver for Cosmos {
    fn init(&mut self, cols: usize, rows: usize) {
        self.intro_fade = 0.0;
        self.state_fade = 0.0;
        self.time_elapsed = 0.0;
        self.last_cols = 0;
        self.last_rows = 0;
        let _ = (cols, rows);
    }

    fn update_frame_time(&mut self, dt: Duration) {
        physics::update_frame_time(self, dt);
    }

    fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        physics::update_life(self, dt, cols, rows);
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        render::draw_life(self, grid, cols, rows);
    }
}

#[cfg(test)]
#[cfg(test)]
#[path = "cosmos_tests.rs"]
mod tests;
