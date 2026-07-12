// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

//! Vendored subset of the `library` crate, scoped to what screensaver-security
//! actually uses. See `LIBRARY_VENDORED.md` for the full list of source
//! provenance and what was deliberately omitted.
//!
//! Original (pre-vendoring) location: /home/jeryd/library (crateria/library).
//! Vendored on: 2026-06-12 from tag v2026.6.10.
//! License: MIT (see top-level LICENSE file).
//!
//! ## Modules
//!
//! - [`cell_renderer`] — terminal grid → BGRA framebuffer rasterization
//! - [`core`] — shared primitives (logo block, palette, screensaver traits)
//! - [`discovery`] — locate installed screensaver plugins on disk
//! - [`launcher`] — validate and spawn plugin binaries
//! - [`plugin_session`] — load, tick, and render a plugin for presentation
//! - [`toolkit`] — host queries (system info, theme, platform metadata)
//! - [`trance_runner`] — fullscreen plugin runner for manual testing

pub mod apps;
pub mod caption_overlay;
pub mod cell_renderer;
pub mod core;
pub mod discovery;
pub mod fps_overlay;
pub mod launcher;
pub mod plugin_session;
pub mod sandbox;
pub mod toolkit;
pub mod trance_runner;

// Tests can run with `cargo test -- --nocapture` to see tracing output.
