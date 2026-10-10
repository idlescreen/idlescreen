// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Test suite shim for the ASCII screensaver.
//!
//! Organizes tests into modular files to keep `src/ascii/` directory density
//! under the eight-file limit while keeping each test file under 256 lines.
//!
//! Submodules:
//! - `ascii_tests`: lifecycle, effects rotation, and screensaver transitions.
//! - `art_tests`: art resolution, branding targets, and dimensions.
//! - `paint_tests`: ANSI frame parsing, colors, and smooth visual transitions.

#[path = "ascii_tests.rs"]
mod ascii_tests;

#[path = "art_tests.rs"]
mod art_tests;

#[path = "paint_tests.rs"]
mod paint_tests;

#[path = "transition_tests.rs"]
mod transition_tests;
