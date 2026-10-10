//! Rain on a still pond — continuous rings, interference, weather states.
//! OS logo sits in the water and warps when rings pass through it.

mod draw;
mod physics;
// `ripple::ripple` holds `Ripple::new` plus its helpers. The Screensaver
// trait impl lives in the sibling `screensaver_impl.rs`.
#[allow(clippy::module_inception)]
mod ripple;
mod screensaver_impl;
mod types;
pub use physics::update;

pub use types::{DelayedRing, Drop, Ring, Splash, Weather};

pub struct Ripple {
    pub(crate) rng: crate::runner::LcgRng,
    pub(crate) time: f32,
    pub(crate) intro_fade: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,
    pub(crate) rings: Vec<Ring>,
    pub(crate) drops: Vec<Drop>,
    pub(crate) splashes: Vec<Splash>,
    pub(crate) delayed: Vec<DelayedRing>,
    pub(crate) rain_timer: f32,
    pub(crate) wind: f32,
    pub(crate) weather: Weather,
    pub(crate) weather_timer: f32,
    pub(crate) weather_intensity: f32,
    pub(crate) on_battery: bool,
    pub(crate) quality_scale: f32,
    pub(crate) frame_time_ema: f32,
    pub(crate) target_frame_time: f32,
    pub(crate) sys_timer: f32,
    pub(crate) accent: (u8, u8, u8),
    pub(crate) logo_text: String,
    pub(crate) surface_phase: f32,
    /// Ring-energy accumulation scratch — reused across frames (draw is
    /// `&self`, so interior mutability). Sized cols*rows on demand.
    pub(crate) accum: std::cell::RefCell<Vec<f32>>,
    pub(crate) accent_a: std::cell::RefCell<Vec<f32>>,
}

#[cfg(test)]
#[path = "ripple_tests.rs"]
mod tests;
