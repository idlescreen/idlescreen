// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Native Wayland presentation layer with wp_viewporter hardware scaling,
//! fractional scaling, and direct font rasterization into wl_shm buffers.

pub mod blit;
pub mod dpms;
pub mod fractional;
pub mod surface;
pub mod viewport;

pub use blit::blit_cells_to_argb;
pub use dpms::{DpmsController, OutputPowerMode};
pub use fractional::calculate_buffer_size;
pub use surface::{OutputInfo, OutputRegistry, PresentationSession, SessionState};
pub use viewport::SurfaceViewport;
