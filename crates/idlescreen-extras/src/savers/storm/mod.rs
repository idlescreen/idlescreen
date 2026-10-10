//! Consolidated storm screensaver effect module.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).

pub mod draw;
pub mod physics;
pub(crate) mod screensaver_impl;
// `storm::storm` holds the `Storm` constructor and helpers.
// The Screensaver trait impl lives in the sibling `screensaver_impl.rs`.
#[allow(clippy::module_inception)]
pub(crate) mod storm;
pub mod types;

#[allow(unused_imports)]
pub use self::types::{
    Animal, AnimalState, AnimalType, BirdState, Drop, LogoCell, Phase, SceneryCell, Splash, Star,
};

pub struct Storm {
    pub(crate) rng: crate::runner::LcgRng,
    pub(crate) stars: Vec<Star>,
    pub(crate) logo_cells: Vec<LogoCell>,
    pub(crate) drops: Vec<Drop>,
    pub(crate) splashes: Vec<Splash>,
    pub(crate) phase: Phase,
    pub(crate) phase_timer: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,
    pub(crate) drop_count_opt: u32,
    pub(crate) assemble_speed_opt: u32,

    // Live system dynamics
    pub(crate) sys_refresh_timer: f32,
    pub(crate) mem_pressure: f32,
    pub(crate) cpu_load: f32,
    pub(crate) _host_bias: f32,
    pub(super) on_battery: bool,
    pub(super) frame_time_ema: f32,
    pub(super) quality_scale: f32,
    pub(super) target_frame_time: f32,

    // Puddle accumulation
    pub(crate) puddle: Vec<f32>,
    pub(crate) puddle_color: Vec<(u8, u8, u8)>,

    // Wind dynamics
    pub(crate) wind: f32,

    // Lightning
    pub(crate) lightning_timer: f32,
    pub(crate) lightning_flash: f32,
    pub(crate) lightning_bolts: Vec<Vec<(usize, usize)>>,
    pub(crate) lightning_is_background: bool,
    pub(crate) lightning_delay: f32,

    // Scenery
    pub(crate) bg_cells: Vec<SceneryCell>,
    pub(crate) mid_scenery: Vec<SceneryCell>,
    pub(crate) fg_scenery: Vec<SceneryCell>,

    // Bird state
    pub(crate) bird_x: f32,
    pub(crate) bird_y: f32,
    pub(crate) bird_state: BirdState,
    pub(crate) bird_timer: f32,
    pub(crate) bird_wing_flap: bool,
    pub(crate) bird_vx: f32,
    pub(crate) bird_vy: f32,
    pub(crate) bird_perch_x: f32,
    pub(crate) bird_perch_y: f32,
    pub(crate) perch_points: Vec<(usize, usize)>,

    // Active Animal
    pub(crate) active_animal: Option<Animal>,
    pub(crate) animal_spawn_timer: f32,

    // Subtitles
    pub(crate) subtitle: String,
    pub(crate) subtitle_timer: f32,
    pub(crate) time_elapsed: f32,
    pub(crate) cached_accent: (u8, u8, u8),
    /// 0→1 fade-in after init / resize (~0.45s)
    pub(crate) intro_fade: f32,
    /// Smoothed wind target (actual wind eases toward this)
    pub(crate) wind_target: f32,
}

#[cfg(test)]
#[path = "storm_tests.rs"]
mod tests;
