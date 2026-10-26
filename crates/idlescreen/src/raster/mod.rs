// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Pure-CPU rasterizer: terminal cell grid → pixel buffer.
//!
//! This tree holds the presentation math that turns the ASCII engine's cell
//! grid into pixels: the glyph font, the ARGB blitter and the high-resolution
//! sub-cell overlay. None of it talks to Wayland — the compositor-facing
//! layer lives in `wayland/`, which consumes these pages.
//!
//! It is deliberately free of the daemon's product dependencies (D-Bus,
//! Wayland protocol bindings) so that a headless renderer can link the scenes
//! and the rasterizer without pulling in a compositor stack.

pub mod blit;
pub mod damage;
pub mod glyph_font;
pub mod glyph_style;
pub mod subcell_overlay;

pub use blit::blit_cells_to_argb;
pub use damage::compute_cell_damage_rect;
pub use glyph_font::{GLYPH_HEIGHT, GLYPH_WIDTH, get_glyph_bitmap};
pub use glyph_style::GlyphFontStyle;
