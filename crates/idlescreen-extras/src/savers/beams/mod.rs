mod light;
mod pacing;
pub mod physics;
mod sys;
mod types;

use physics::{advection, motion};

pub use types::{DustParticle, Spotlight, Star, smoothstep};

use crate::runner::Screensaver;
use crate::runner::{LcgRng, TerminalCell};
use std::time::Duration;

use crate::runner::{get_system_info, query_current_palette};

/// Max beams allowed to rest at the same time (never freeze the whole field).
const MAX_CALM_BEAMS: usize = 2;

pub struct Beams {
    pub(super) rng: LcgRng,
    pub(super) stars: Vec<Star>,
    pub(super) particles: Vec<DustParticle>,
    pub(super) spotlights: Vec<Spotlight>,
    pub(super) time_elapsed: f32,
    pub(super) last_cols: usize,
    pub(super) last_rows: usize,
    pub(super) twinkle_stars_opt: u32,

    pub(super) sys_refresh_timer: f32,
    pub(super) mem_pressure: f32,
    pub(super) cpu_load: f32,
    pub(super) host_bias: f32,
    pub(super) on_battery: bool,
    pub(super) frame_time_ema: f32,
    pub(super) quality_scale: f32,
    pub(super) target_frame_time: f32,
    pub(super) logo_text: String,
    pub(super) cached_accent: (u8, u8, u8),
    pub(super) current_accent: (f32, f32, f32),

    /// 0→1 fade-in after init / resize (~0.45s)
    pub(super) intro_fade: f32,
}

impl Default for Beams {
    fn default() -> Self {
        Self::new()
    }
}

impl Beams {
    pub fn new() -> Self {
        let beam_count: u32 = 4;
        let twinkle_stars_opt: u32 = 1;

        let sys = get_system_info();
        let host_bias = sys.hostname.chars().map(|c| c as u32).sum::<u32>() as f32 / 1000.0 % 1.0;
        let mem_pressure = sys.mem_used_pct / 100.0;
        let cpu_load = (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
        let on_battery = sys.power_status.contains("Battery");
        let logo_text = sys.logo_text.clone();
        let accent = query_current_palette().accent;

        let spotlights = light::create_spotlights(beam_count, accent, host_bias);

        Self {
            rng: LcgRng::from_env_or_random(),
            stars: Vec::new(),
            particles: Vec::new(),
            spotlights,
            time_elapsed: 0.0,
            last_cols: 0,
            last_rows: 0,
            twinkle_stars_opt,
            sys_refresh_timer: 0.0,
            mem_pressure,
            cpu_load,
            host_bias,
            on_battery,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,
            logo_text,
            cached_accent: accent,
            current_accent: (accent.0 as f32, accent.1 as f32, accent.2 as f32),
            intro_fade: 0.0,
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

impl Screensaver for Beams {
    fn init(&mut self, cols: usize, rows: usize) {
        self.intro_fade = 0.0;
        self.last_cols = cols;
        self.last_rows = rows;
        self.stars.clear();
        self.particles.clear();
        self.time_elapsed = 0.0;
        // Reseed per-beam rest timers so a resize doesn't re-sync them.
        for (i, spot) in self.spotlights.iter_mut().enumerate() {
            spot.is_calm = false;
            spot.motion_blend = 1.0;
            spot.motion_timer = 3.0 + i as f32 * 2.8 + self.rng.next_f32() * 2.5;
        }
    }

    fn update_frame_time(&mut self, dt: Duration) {
        let mut p = pacing::FramePacing {
            frame_time_ema: self.frame_time_ema,
            target_frame_time: self.target_frame_time,
            quality_scale: self.quality_scale,
        };
        pacing::update_frame_time(&mut p, self.time_elapsed, self.on_battery, dt);
        self.frame_time_ema = p.frame_time_ema;
        self.target_frame_time = p.target_frame_time;
        self.quality_scale = p.quality_scale;
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

        motion::update_beam_motion(&mut self.spotlights, &mut self.rng, delta);

        self.sys_refresh_timer += delta;
        if self.sys_refresh_timer >= 1.0 {
            let metrics = sys::poll_sys_info(&mut self.spotlights, self.host_bias);
            self.mem_pressure = metrics.mem_pressure;
            self.cpu_load = metrics.cpu_load;
            self.on_battery = metrics.on_battery;
            self.logo_text = metrics.logo_text;
            self.cached_accent = metrics.cached_accent;
            self.sys_refresh_timer = 0.0;
        }

        // Smooth 1-second RGB lerp toward target accent
        let t = (delta / 1.0).clamp(0.0, 1.0);
        self.current_accent.0 += (self.cached_accent.0 as f32 - self.current_accent.0) * t;
        self.current_accent.1 += (self.cached_accent.1 as f32 - self.current_accent.1) * t;
        self.current_accent.2 += (self.cached_accent.2 as f32 - self.current_accent.2) * t;
        if self.spotlights.len() > 1 {
            self.spotlights[1].color_r = self.current_accent.0;
            self.spotlights[1].color_g = self.current_accent.1;
            self.spotlights[1].color_b = self.current_accent.2;
        }

        let bat = if self.on_battery { 0.55 } else { 1.0 };
        // Extra cut when frame budget is tight (quality_scale already drops)
        let max_stars = ((if self.twinkle_stars_opt == 1 {
            (cols * rows / 16).clamp(30, 200)
        } else {
            0
        }) as f32
            * self.quality_scale
            * bat) as usize;
        let max_particles =
            (((cols * rows / 12).clamp(30, 150)) as f32 * self.quality_scale * bat * 0.92) as usize;

        if cols != self.last_cols || rows != self.last_rows {
            self.stars.clear();
            self.particles.clear();
            self.last_cols = cols;
            self.last_rows = rows;
            self.intro_fade = 0.0;
        }

        if self.stars.len() > max_stars {
            self.stars.truncate(max_stars);
        } else if self.stars.len() < max_stars && max_stars > 0 {
            while self.stars.len() < max_stars {
                // ~35% far plane, rest near
                let near = self.rng.next_f32() > 0.35;
                let star = motion::spawn_star(&mut self.rng, self.stars.len(), near);
                self.stars.push(star);
            }
        }

        if self.particles.len() > max_particles {
            self.particles.truncate(max_particles);
        } else if self.particles.len() < max_particles && max_particles > 0 {
            while self.particles.len() < max_particles {
                let near = self.rng.next_f32() > 0.40;
                let dust = motion::spawn_dust(&mut self.rng, near);
                self.particles.push(dust);
            }
        }

        for star in &mut self.stars {
            if star.excitation > 0.0 {
                // Near stars decay faster after a spark
                let decay = if star.layer == 1 { 2.2 } else { 1.4 };
                star.excitation = (star.excitation - delta * decay).max(0.0);
            }
        }

        advection::update_dust_and_stars(
            &mut self.particles,
            &mut self.stars,
            &self.spotlights,
            &mut self.rng,
            cols,
            rows,
            delta,
            self.cached_accent,
            crate::runner::is_secondary_monitor(),
        );

        // Beam phase advance with per-beam bias + per-beam calm easing
        for spot in &mut self.spotlights {
            let ease = smoothstep(0.0, 1.0, spot.motion_blend);
            // Slight hold near amplitude peaks when this beam is calming
            let wave = (spot.phase + spot.phase_offset).sin();
            let hold = 1.0 - 0.35 * (1.0 - ease) * wave.abs();
            spot.phase += spot.speed * spot.speed_bias * delta * ease * hold;
        }
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        physics::draw_impl(
            grid,
            cols,
            rows,
            &self.spotlights,
            &self.stars,
            &self.particles,
            self.twinkle_stars_opt,
            self.time_elapsed,
            &self.logo_text,
            self.cached_accent,
            self.intro_fade,
        );
    }
}

#[cfg(test)]
#[path = "beams_tests/mod.rs"]
mod tests;
