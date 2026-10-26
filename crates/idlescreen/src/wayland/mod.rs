// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Native Wayland presentation layer with wp_viewporter hardware scaling,
//! fractional scaling, and direct font rasterization into wl_shm buffers.

pub mod blit;
pub mod dpms;
pub mod fractional;
pub mod glyph_font;
pub mod presenter;
pub mod subcell_overlay;
pub mod viewport;

pub use blit::blit_cells_to_argb;
pub use dpms::{DpmsController, OutputPowerMode};
pub use fractional::calculate_buffer_size;
pub use presenter::{run_wayland_daemon_presenter, run_wayland_fullscreen_preview};
pub use viewport::SurfaceViewport;
