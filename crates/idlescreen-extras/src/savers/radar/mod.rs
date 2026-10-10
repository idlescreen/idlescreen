mod blips;
mod draw;
mod draw_hud;
pub mod physics;
mod types;

use crate::runner::Screensaver;
use crate::runner::{LcgRng, TerminalCell};
use crate::runner::{get_system_info, query_current_palette};
use std::time::Duration;
use types::{EnemyShip, ExplosionParticle, GoodShip, LaserBeam, RadarJet};

pub struct Radar {
    rng: LcgRng,
    next_enemy_id: u32,
    sweep_angle: f32,
    enemies: Vec<EnemyShip>,
    defenders: Vec<GoodShip>,
    jets: Vec<RadarJet>,
    lasers: Vec<LaserBeam>,
    particles: Vec<ExplosionParticle>,
    shield_health: f32,

    time_elapsed: f32,
    intro_fade: f32,
    on_battery: bool,
    frame_time_ema: f32,
    quality_scale: f32,
    target_frame_time: f32,
    sys_refresh_timer: f32,
    last_cols: usize,
    last_rows: usize,
    cached_accent: (u8, u8, u8),
}

impl Default for Radar {
    fn default() -> Self {
        Self::new()
    }
}

impl Radar {
    pub fn new() -> Self {
        let sys = get_system_info();
        let on_battery = sys.power_status.contains("Battery");
        let accent = query_current_palette().accent;
        Self {
            rng: LcgRng::from_env_or_random(),
            next_enemy_id: 1,
            sweep_angle: 0.0,
            enemies: Vec::new(),
            defenders: Vec::new(),
            jets: Vec::new(),
            lasers: Vec::new(),
            particles: Vec::new(),
            shield_health: 100.0,
            time_elapsed: 0.0,
            intro_fade: 0.0,
            on_battery,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,
            sys_refresh_timer: 0.0,
            last_cols: 0,
            last_rows: 0,
            cached_accent: accent,
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
}

impl Screensaver for Radar {
    fn init(&mut self, cols: usize, rows: usize) {
        self.enemies.clear();
        self.defenders.clear();
        self.jets.clear();
        self.lasers.clear();
        self.particles.clear();
        self.shield_health = 100.0;
        self.sweep_angle = 0.0;
        self.next_enemy_id = 1;
        self.intro_fade = 0.0;
        self.time_elapsed = 0.0;
        self.last_cols = cols;
        self.last_rows = rows;
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

        if self.intro_fade < 1.0 {
            self.intro_fade = (self.intro_fade + delta / 0.45).min(1.0);
        }

        if cols != self.last_cols || rows != self.last_rows {
            self.last_cols = cols;
            self.last_rows = rows;
            self.intro_fade = 0.0;
        }

        self.sys_refresh_timer += delta;
        if self.sys_refresh_timer >= 1.0 {
            let sys = get_system_info();
            self.on_battery = sys.power_status.contains("Battery");
            self.cached_accent = query_current_palette().accent;
            self.sys_refresh_timer = 0.0;
        }

        physics::update_simulation(
            dt,
            cols,
            rows,
            &mut self.rng,
            &mut self.next_enemy_id,
            &mut self.sweep_angle,
            &mut self.enemies,
            &mut self.defenders,
            &mut self.jets,
            &mut self.lasers,
            &mut self.particles,
            &mut self.shield_health,
            self.on_battery,
            self.quality_scale,
        );
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        let palette = query_current_palette();
        draw::draw_simulation(
            grid,
            cols,
            rows,
            self.sweep_angle,
            &self.enemies,
            &self.defenders,
            &self.jets,
            &self.lasers,
            &self.particles,
            self.shield_health,
            &palette,
            self.intro_fade,
            self.cached_accent,
        );
    }
}

#[cfg(test)]
#[path = "radar_tests.rs"]
mod tests;
