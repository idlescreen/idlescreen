// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Native Wayland presentation layer with wp_viewporter hardware scaling,
//! fractional scaling, and direct font rasterization into wl_shm buffers.
//!
//! The rasterizer itself lives in [`crate::raster`]: the glyph font, the
//! ARGB blitter and the sub-cell overlay are pure CPU and know nothing about
//! Wayland, so a headless renderer can use them without a compositor stack.

pub mod dpms;
pub mod fractional;
pub mod presenter;
pub mod viewport;

pub use crate::raster::blit::blit_cells_to_argb;
pub use dpms::{DpmsController, OutputPowerMode};
pub use fractional::calculate_buffer_size;
pub use presenter::{run_wayland_daemon_presenter, run_wayland_fullscreen_preview};
pub use viewport::SurfaceViewport;
