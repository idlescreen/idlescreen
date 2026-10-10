//! Physics simulation and collision detection for Radar.

pub mod collision;
pub mod jets;
pub mod particles;
#[allow(clippy::module_inception)]
pub mod physics;

pub use physics::update_simulation;
