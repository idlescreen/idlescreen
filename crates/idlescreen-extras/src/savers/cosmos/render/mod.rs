//! Terminal rendering for the cosmos universe lifecycle.
//!
//! Submodules cover particle trails, gravitational waves, the Big Bang shell,
//! stellar coronae, black-hole accretion disks, gravity seeds, and the
//! per-state `draw_life` orchestrator. Black-hole disk drawing is shared
//! between seed rendering and singularity/collapse views.

mod big_bang_shell;
mod black_hole_disk;
mod grav_wave;
mod life;
mod particles;
mod seeds;
mod star_corona;

pub use life::draw_life;

// Re-export surface kept intentionally small: callers use `render::draw_life` and
// helper draw fns only where physics or tests need direct access. Internal
// modules (star corona, black hole disk) stay crate-private unless re-exported above.

// Module boundary: rendering is read-only with respect to simulation state.
