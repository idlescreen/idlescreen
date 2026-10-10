//! Consolidated gnats screensaver effect module.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).

pub mod physics;
mod pop;
mod population;
mod render;
mod render_effects;
mod types;

pub use types::{Attractor, Firefly, KillSpark, Star};

use crate::runner::Screensaver;
use crate::runner::{LcgRng, TerminalCell};
use crate::runner::{get_system_info, query_current_palette};
use std::time::Duration;

pub struct Gnats {
    pub(crate) rng: LcgRng,
    pub(crate) fireflies: Vec<Firefly>,
    pub(crate) attractors: Vec<Attractor>,
    pub(crate) stars: Vec<Star>,
    pub(crate) kill_sparks: Vec<KillSpark>,
    pub(crate) time_elapsed: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,
    pub(crate) logo_excitation: Vec<f32>,
    pub(super) on_battery: bool,
    pub(super) frame_time_ema: f32,
    pub(super) quality_scale: f32,
    pub(super) target_frame_time: f32,
    pub(crate) sys_refresh_timer: f32,
    pub(crate) mem_pressure: f32,
    pub(crate) cpu_load: f32,
    pub(crate) logo_text: String,
    pub(crate) accent: (u8, u8, u8),
    /// 0→1 fade-in after init / resize (~0.45s)
    pub(crate) intro_fade: f32,
    /// Breathing multiplier for attractor pull (calm ↔ tense swarm).
    pub(crate) attractor_strength: f32,
    /// Rare window when predator-prey chase is active.
    pub(crate) predator_active: bool,
    pub(crate) predator_timer: f32,
}

impl Default for Gnats {
    fn default() -> Self {
        Self::new()
    }
}

impl Gnats {
    pub fn new() -> Self {
        let rng = LcgRng::from_env_or_random();
        let sys = get_system_info();
        let on_battery = sys.power_status.contains("Battery");
        let accent = query_current_palette().accent;
        Self {
            rng,
            fireflies: Vec::new(),
            attractors: Vec::new(),
            stars: Vec::new(),
            kill_sparks: Vec::new(),
            time_elapsed: 0.0,
            last_cols: 0,
            last_rows: 0,
            logo_excitation: Vec::new(),
            on_battery,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,
            sys_refresh_timer: 0.0,
            mem_pressure: sys.mem_used_pct / 100.0,
            cpu_load: (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0),
            logo_text: sys.logo_text,
            accent,
            intro_fade: 0.0,
            attractor_strength: 1.0,
            predator_active: false,
            predator_timer: 8.0,
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

impl Screensaver for Gnats {
    fn init(&mut self, cols: usize, rows: usize) {
        self.intro_fade = 0.0;
        self.last_cols = cols;
        self.last_rows = rows;
        self.fireflies.clear();
        self.stars.clear();
        self.kill_sparks.clear();
        self.attractors.clear();
        self.time_elapsed = 0.0;
        self.attractor_strength = 1.0;
        self.predator_active = false;
        self.predator_timer = 8.0 + self.rng.next_f32() * 6.0;
    }

    fn update_frame_time(&mut self, dt: Duration) {
        self.update_frame_time_impl(dt);
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

        // Calm ↔ tense attractor breathing (never freezes the field)
        let breathe = 0.55 + 0.45 * (0.5 + 0.5 * (self.time_elapsed * 0.18).sin());
        self.attractor_strength = breathe;

        // Rare predator windows: mostly peaceful drift, brief chase storms
        self.predator_timer -= delta;
        if self.predator_timer <= 0.0 {
            if self.predator_active {
                self.predator_active = false;
                self.predator_timer = 10.0 + self.rng.next_f32() * 18.0;
            } else {
                self.predator_active = true;
                self.predator_timer = 2.5 + self.rng.next_f32() * 3.5;
            }
        }

        self.sys_refresh_timer += delta;
        if self.sys_refresh_timer >= 1.0 {
            let sys = get_system_info();
            self.mem_pressure = sys.mem_used_pct / 100.0;
            self.cpu_load = (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
            self.on_battery = sys.power_status.contains("Battery");
            self.logo_text = sys.logo_text;
            self.accent = query_current_palette().accent;
            self.sys_refresh_timer = 0.0;
        }

        // Initialize particles and attractors if grid size changes
        if cols != self.last_cols || rows != self.last_rows {
            self.last_cols = cols;
            self.last_rows = rows;
            self.intro_fade = 0.0;

            self.logo_excitation =
                crate::runner::place_centered_logo(cols, rows, &self.logo_text, None)
                    .map(|logo| vec![0.0; logo.width * logo.height])
                    .unwrap_or_default();

            pop::create_attractors(&mut self.attractors, cols, rows, self.accent);

            self.fireflies.clear();
            self.stars.clear();
            self.kill_sparks.clear();
        }

        self.adjust_populations(cols, rows);

        let cols_f = cols as f32;
        let rows_f = rows as f32;

        physics::update_attractors(&mut self.attractors, self.time_elapsed, cols_f, rows_f);
        physics::decay_logo_excitations(&mut self.logo_excitation, delta);

        let dead_indices = physics::compute_firefly_forces_and_update(
            &mut self.fireflies,
            &self.attractors,
            self.time_elapsed,
            cols_f,
            rows_f,
            &mut self.rng,
            delta,
            self.attractor_strength,
            self.predator_active,
        );

        // Process dead fireflies (remove, trigger explosions, and respawn)
        if !dead_indices.is_empty() {
            let mut unique_dead = dead_indices;
            unique_dead.sort_unstable();
            unique_dead.dedup();

            for &idx in unique_dead.iter().rev() {
                if idx < self.fireflies.len() {
                    let dead = self.fireflies.remove(idx);

                    // Spawn a colorful neon spark explosion burst
                    for _ in 0..12 {
                        let angle = self.rng.next_range(0.0, std::f32::consts::TAU);
                        let speed = self.rng.next_range(8.0, 22.0);
                        self.kill_sparks.push(KillSpark {
                            x: dead.x,
                            y: dead.y,
                            vx: angle.cos() * speed,
                            vy: angle.sin() * speed * 0.5,
                            color: dead.color,
                            life: self.rng.next_range(0.5, 1.2),
                        });
                    }

                    // Respawn a new firefly on the border to replace the population
                    self.spawn_new_firefly(cols, rows);
                }
            }
        }

        physics::update_kill_sparks(&mut self.kill_sparks, delta);
        physics::update_stars(&mut self.stars, &self.fireflies, delta, cols_f, rows_f);
        physics::update_logo_excitations(
            &mut self.logo_excitation,
            &self.fireflies,
            cols,
            rows,
            &self.logo_text,
        );
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        render::draw_gnats(self, grid, cols, rows);
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
