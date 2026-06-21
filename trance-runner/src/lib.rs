//! Vendored subset of the `library` crate, scoped to what screensaver-security
//! actually uses. See `LIBRARY_VENDORED.md` for the full list of source
//! provenance and what was deliberately omitted.
//!
//! Original (pre-vendoring) location: /home/jeryd/library (local76/library).
//! Vendored on: 2026-06-12 from tag v2026.6.10.
//! License: MIT (see top-level LICENSE file).

pub mod apps;
pub mod cell_renderer;
pub mod core;
pub mod discovery;
pub mod caption_overlay;
pub mod fps_overlay;
pub mod launcher;
pub mod plugin_meta;
pub mod plugin_session;
pub mod toolkit;
pub mod trance_runner;
