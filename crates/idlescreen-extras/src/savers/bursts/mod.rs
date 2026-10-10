//! Consolidated bursts screensaver effect module — hero visual polish.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).

use crate::runner::LcgRng;
use crate::runner::{get_system_info, query_current_palette};

pub mod draw;
pub mod drawing;
pub mod physics;
pub mod types;
pub mod update;

use types::{Rocket, Particle, Explosion, Star};

pub struct Bursts {
    pub(crate) rng: LcgRng,
    pub(crate) rockets: Vec<Rocket>,
    pub(crate) particles: Vec<Particle>,
    pub(crate) explosions: Vec<Explosion>,
    pub(crate) stars: Vec<Star>,
    pub(crate) skyline: Vec<usize>, // Height of building at each column
    pub(crate) skyline_windows: Vec<bool>, // Whether window is lit at grid cell (r * cols + c)
    pub(crate) time_elapsed: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,
    pub(crate) launch_rate_opt: u32,
    pub(crate) skyline_style_opt: u32,
    pub(crate) logo_text: String,
    pub(crate) accent: (u8, u8, u8),
    pub(crate) target_accent: (u8, u8, u8),
    pub(crate) current_accent: (f32, f32, f32),

    // Live system dynamics
    pub(crate) sys_refresh_timer: f32,
    pub(crate) mem_pressure: f32,
    pub(crate) cpu_load: f32,
    pub(crate) host_bias: f32,
    pub(crate) on_battery: bool,
    pub(crate) frame_time_ema: f32,
    pub(crate) quality_scale: f32,
    pub(crate) target_frame_time: f32,

    /// 0→1 fade-in after init / resize (~0.45s)
    pub(crate) intro_fade: f32,
    /// Cooldown before next rocket can launch (staggers volleys).
    pub(crate) launch_cooldown: f32,
    /// Quiet sky period with no new launches (breathing between waves).
    pub(crate) quiet_timer: f32,
    /// Rockets launched in the current wave (triggers quiet after a few).
    pub(crate) wave_launches: u32,
}

impl Default for Bursts {
    fn default() -> Self {
        Self::new()
    }
}

impl Bursts {
    pub fn new() -> Self {
        let launch_rate_opt: u32 = 1;
        let skyline_style_opt: u32 = 0;

        let sys = get_system_info();
        let logo_text = sys.logo_text.clone();
        let host_bias = sys.hostname.chars().map(|c| c as u32).sum::<u32>() as f32 / 1000.0 % 1.0;
        let on_battery = sys.power_status.contains("Battery");

        Self {
            rng: LcgRng::from_env_or_random(),
            rockets: Vec::new(),
            particles: Vec::new(),
            explosions: Vec::new(),
            stars: Vec::new(),
            skyline: Vec::new(),
            skyline_windows: Vec::new(),
            time_elapsed: 0.0,
            last_cols: 0,
            last_rows: 0,
            launch_rate_opt,
            skyline_style_opt,
            logo_text,
            accent: query_current_palette().accent,
            target_accent: query_current_palette().accent,
            current_accent: (
                query_current_palette().accent.0 as f32,
                query_current_palette().accent.1 as f32,
                query_current_palette().accent.2 as f32,
            ),
            sys_refresh_timer: 0.0,
            mem_pressure: sys.mem_used_pct / 100.0,
            cpu_load: (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0),
            host_bias,
            on_battery,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,
            intro_fade: 0.0,
            // Stagger first launch so we don't open with a full volley.
            launch_cooldown: 0.35 + host_bias * 0.4,
            quiet_timer: 0.0,
            wave_launches: 0,
        }
    }

    pub(crate) fn generate_skyline(&mut self, cols: usize, rows: usize) {
        self.skyline = vec![0; cols];
        self.skyline_windows = vec![false; cols * rows];

        if self.skyline_style_opt == 1
            || rows < 4
            || cols == 0
            || crate::runner::is_secondary_monitor()
        {
            return; // Empty sky, too small terminal, or secondary monitor
        }

        let primary = crate::runner::get_primary_monitor_bounds(cols, rows);
        let mut c = primary.start_col;
        while c < primary.end_col {
            let building_w = self.rng.next_usize(6) + 3; // 3 to 8 cols wide
            let building_h = self.rng.next_usize(rows / 4) + 3; // building height

            for i in 0..building_w {
                if c + i < primary.end_col {
                    self.skyline[c + i] = building_h;

                    // Windows in this building
                    for r in 0..building_h {
                        let gy = rows.saturating_sub(1).saturating_sub(r);
                        if self.rng.next_bool(0.12) {
                            self.skyline_windows[gy * cols + (c + i)] = true;
                        }
                    }
                }
            }
            c += building_w + self.rng.next_usize(2); // gap between buildings
        }
    }

    /// Pin the saver into a stable state for bench harness measurements.
    /// See `cosmos::Cosmos::prepare_for_bench` for the rationale; the
    /// important bit is `sys_refresh_timer = -1000.0` to suppress the
    /// slow system-info probe inside `update()`.
    pub fn prepare_for_bench(&mut self, _cols: usize, _rows: usize) {
        self.sys_refresh_timer = -1000.0;
        // Do NOT stamp `last_cols`/`last_rows` here. `resize_if_needed`
        // only re-initializes when the grid size actually changes, so
        // pre-stamping the target size meant the first `update()` saw
        // "nothing changed", skipped `generate_skyline`, and left
        // `skyline` empty — the next `draw()` then indexed
        // `skyline[sx]` and panicked. That panic was live in CI since
        // the `tick` bench landed; `perf.yml` hid it behind `|| true`.
        //
        // Zeroing the cached size instead lets the first `update()`
        // drive the real production init path.
        self.last_cols = 0;
        self.last_rows = 0;
    }
}

#[cfg(test)]
#[path = "bursts_tests.rs"]
mod tests;
