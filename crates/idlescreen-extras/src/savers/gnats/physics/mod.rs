//! Physics simulation and force calculations for Gnats.

pub mod forces;
pub mod simulation;

#[cfg(test)]
mod tests;

pub use simulation::*;
